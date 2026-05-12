use std::fs;
use std::io;
use std::path::Path;

use serde_json::{Value, json};

use crate::engine::archetype_library::ArchetypeLibrary;
use crate::engine::asset_tools::AssetTools;
use crate::engine::build_placement::{BuildFootprint, BuildPlacement, PlacementResult};
use crate::engine::component::{Component, component_from_data, default_component};
use crate::engine::spatial_index::{SpatialEntry, SpatialIndex};
use crate::entities::game_object::GameObject;
use crate::map::flow_field::FlowField;
use crate::map::grid::Grid;
use crate::systems::rts_system::RTSSystem;

#[derive(Debug, Clone, Default)]
pub struct GameAPI;

impl GameAPI {
    pub fn find<'a>(entities: &'a [GameObject], name: &str) -> Option<&'a GameObject> {
        entities.iter().find(|entity| entity.name == name)
    }

    pub fn find_mut<'a>(entities: &'a mut [GameObject], name: &str) -> Option<&'a mut GameObject> {
        entities.iter_mut().find(|entity| entity.name == name)
    }

    pub fn find_by_id(entities: &[GameObject], entity_id: u64) -> Option<&GameObject> {
        entities.iter().find(|entity| entity.id == entity_id)
    }

    pub fn find_with_tag<'a>(entities: &'a [GameObject], tag: &str) -> Vec<&'a GameObject> {
        entities.iter().filter(|entity| entity.tag == tag).collect()
    }

    pub fn find_with_component<'a>(
        entities: &'a [GameObject],
        component_type: &str,
    ) -> Vec<&'a GameObject> {
        entities
            .iter()
            .filter(|entity| entity.get_component(component_type).is_some())
            .collect()
    }

    pub fn create_game_object(entities: &mut Vec<GameObject>, name: &str, x: f64, y: f64) -> u64 {
        let entity = GameObject::new(x, y, Some(name.to_string()));
        let id = entity.id;
        entities.push(entity);
        id
    }

    pub fn create_unit(entities: &mut Vec<GameObject>, name: &str, x: f64, y: f64) -> u64 {
        let entity = GameObject::new_unit(x, y, Some(name.to_string()));
        let id = entity.id;
        entities.push(entity);
        id
    }

    pub fn spawn_archetype(
        entities: &mut Vec<GameObject>,
        library: &ArchetypeLibrary,
        key: &str,
        x: f64,
        y: f64,
        team_id: Option<i64>,
    ) -> Option<u64> {
        let entity = library.instantiate(key, x, y, team_id)?;
        let id = entity.id;
        entities.push(entity);
        Some(id)
    }

    pub fn instantiate_prefab(
        entities: &mut Vec<GameObject>,
        prefab_path: impl AsRef<Path>,
        x: f64,
        y: f64,
    ) -> io::Result<u64> {
        let data = AssetTools::read_json(prefab_path)?;
        let entity_data = data.get("entity").cloned().unwrap_or(Value::Null);
        let mut entity = GameObject::from_data(&entity_data, false);
        entity.x = x;
        entity.y = y;
        entity.sync_to_components();
        let id = entity.id;
        entities.push(entity);
        Ok(id)
    }

    pub fn destroy(entities: &mut Vec<GameObject>, entity_id: u64) -> bool {
        let before = entities.len();
        entities.retain(|entity| entity.id != entity_id);
        before != entities.len()
    }

    pub fn set_position(entity: &mut GameObject, x: f64, y: f64) {
        entity.x = x;
        entity.y = y;
        entity.sync_to_components();
    }

    pub fn set_x(entity: &mut GameObject, x: f64) {
        Self::set_position(entity, x, entity.y);
    }

    pub fn set_y(entity: &mut GameObject, y: f64) {
        Self::set_position(entity, entity.x, y);
    }

    pub fn translate(entity: &mut GameObject, dx: f64, dy: f64) {
        Self::set_position(entity, entity.x + dx, entity.y + dy);
    }

    pub fn move_x(entity: &mut GameObject, amount: f64) {
        Self::translate(entity, amount, 0.0);
    }

    pub fn move_y(entity: &mut GameObject, amount: f64) {
        Self::translate(entity, 0.0, amount);
    }

    pub fn set_scale(entity: &mut GameObject, scale_x: f64, scale_y: f64) {
        entity.scale_x = scale_x;
        entity.scale_y = scale_y;
        entity.sync_to_components();
    }

    pub fn scale_by(entity: &mut GameObject, scale_x: f64, scale_y: f64) {
        Self::set_scale(entity, entity.scale_x * scale_x, entity.scale_y * scale_y);
    }

    pub fn set_size(entity: &mut GameObject, width: f64, height: f64) {
        entity.width = width.max(0.01);
        entity.height = height.max(0.01);
        entity.sync_to_components();
    }

    pub fn set_rotation(entity: &mut GameObject, rotation: f64) {
        entity.rotation = rotation;
        entity.sync_to_components();
    }

    pub fn rotate_by(entity: &mut GameObject, amount: f64) {
        Self::set_rotation(entity, entity.rotation + amount);
    }

    pub fn look_at(entity: &mut GameObject, x: f64, y: f64) {
        let dx = x - entity.x;
        let dy = y - entity.y;
        if dx.abs() + dy.abs() > f64::EPSILON {
            Self::set_rotation(entity, dy.atan2(dx).to_degrees());
        }
    }

    pub fn move_to(entity: &mut GameObject, x: f64, y: f64) {
        if let Some(nav) = entity.get_component_mut("NavAgent") {
            nav.nav_set_destination(x, y);
        } else {
            entity.path = vec![(x, y)];
            entity.command = "MOVE".to_string();
            entity.state = "MOVING".to_string();
        }
    }

    pub fn issue_attack_move(entity: &mut GameObject, x: f64, y: f64) {
        entity.attack_move_target = Some((x, y));
        Self::move_to(entity, x, y);
        entity.command = "ATTACK_MOVE".to_string();
        entity.state = "MOVING".to_string();
    }

    pub fn assign_squad(entity: &mut GameObject, squad_id: &str, slot: i64, role: &str) -> bool {
        if entity.get_component("SquadMember").is_none() {
            entity.add_component(default_component("SquadMember").expect("SquadMember"));
        }
        let Some(squad) = entity.get_component_mut("SquadMember") else {
            return false;
        };
        squad.set("squad_id", json!(squad_id));
        squad.set("slot", json!(slot.max(0)));
        squad.set("role", json!(role));
        true
    }

    pub fn query_radius<'a>(
        entities: &'a [GameObject],
        x: f64,
        y: f64,
        radius: f64,
        tag: Option<&str>,
        component_type: Option<&str>,
    ) -> Vec<&'a GameObject> {
        entities
            .iter()
            .filter(|entity| entity.enabled)
            .filter(|entity| tag.is_none_or(|tag| entity.tag == tag))
            .filter(|entity| {
                component_type
                    .is_none_or(|component_type| entity.get_component(component_type).is_some())
            })
            .filter(|entity| ((entity.x - x).powi(2) + (entity.y - y).powi(2)).sqrt() <= radius)
            .collect()
    }

    pub fn nearest<'a>(
        entities: &'a [GameObject],
        x: f64,
        y: f64,
        tag: Option<&str>,
        component_type: Option<&str>,
        max_distance: Option<f64>,
    ) -> Option<&'a GameObject> {
        let radius = max_distance.unwrap_or(f64::INFINITY);
        Self::query_radius(entities, x, y, radius, tag, component_type)
            .into_iter()
            .min_by(|a, b| {
                let da = ((a.x - x).powi(2) + (a.y - y).powi(2)).sqrt();
                let db = ((b.x - x).powi(2) + (b.y - y).powi(2)).sqrt();
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn query_radius_indexed(
        index: &SpatialIndex,
        x: f64,
        y: f64,
        radius: f64,
        tag: Option<&str>,
        layer: Option<&str>,
    ) -> Vec<SpatialEntry> {
        index.query_radius(x, y, radius, tag, layer)
    }

    pub fn nearest_indexed(
        index: &SpatialIndex,
        x: f64,
        y: f64,
        radius: f64,
        tag: Option<&str>,
        layer: Option<&str>,
    ) -> Option<SpatialEntry> {
        index.nearest(x, y, radius, tag, layer)
    }

    pub fn flow_field_path(
        grid: &Grid,
        start: (i32, i32),
        goal: (i32, i32),
        max_steps: usize,
    ) -> Vec<(f64, f64)> {
        FlowField::build(grid, goal, grid.width.saturating_mul(grid.height).max(1))
            .map(|field| {
                field
                    .path_from(grid, start, max_steps)
                    .into_iter()
                    .map(|(x, y)| (x as f64, y as f64))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn can_place_building(
        grid: &Grid,
        entities: &[GameObject],
        cell: (i32, i32),
        footprint: &BuildFootprint,
        team_id: Option<i64>,
    ) -> PlacementResult {
        BuildPlacement::validate(grid, entities, cell, footprint, team_id)
    }

    pub fn add_component<'a>(
        entity: &'a mut GameObject,
        component_name: &str,
        data: Option<Value>,
    ) -> Option<&'a mut Component> {
        if let Some(payload) = data {
            let mut payload = payload;
            if let Some(map) = payload.as_object_mut() {
                map.entry("component_type".to_string())
                    .or_insert(json!(component_name));
            }
            if let Some(component) = component_from_data(&payload) {
                return Some(entity.add_component(component));
            }
        }
        let component = default_component(component_name)?;
        Some(entity.add_component(component))
    }

    pub fn remove_component(entity: &mut GameObject, component_type: &str) {
        entity.remove_component(component_type);
    }

    pub fn set_component_value(
        entity: &mut GameObject,
        component_type: &str,
        key: &str,
        value: Value,
    ) -> bool {
        let Some(component) = entity.get_component_mut(component_type) else {
            return false;
        };
        component.set(key, value);
        entity.sync_from_components();
        true
    }

    pub fn get_component_value(
        entity: &GameObject,
        component_type: &str,
        key: &str,
    ) -> Option<Value> {
        entity
            .get_component(component_type)
            .and_then(|component| component.get(key))
            .cloned()
    }

    pub fn spawn_sprite_entity(
        entities: &mut Vec<GameObject>,
        name: &str,
        sprite_name: &str,
        x: f64,
        y: f64,
    ) -> u64 {
        let mut entity = GameObject::new(x, y, Some(name.to_string()));
        entity.sprite_name = Some(sprite_name.to_string());
        if let Some(sprite) = entity.get_component_mut("SpriteRenderer") {
            sprite.set("sprite_name", json!(sprite_name));
            sprite.set("visible", json!(true));
        }
        entity.sync_to_components();
        let id = entity.id;
        entities.push(entity);
        id
    }

    pub fn add_audio_source(
        entity: &mut GameObject,
        audio_name: &str,
        play_on_start: bool,
    ) -> bool {
        if entity.get_component("AudioSource").is_none() {
            entity.add_component(default_component("AudioSource").expect("AudioSource"));
        }
        let Some(audio) = entity.get_component_mut("AudioSource") else {
            return false;
        };
        audio.set("audio_name", json!(audio_name));
        audio.set("play_on_start", json!(play_on_start));
        true
    }

    pub fn damage(entity: &mut GameObject, amount: f64) -> bool {
        let Some(health) = entity.get_component_mut("Health") else {
            return false;
        };
        health.take_damage(amount);
        true
    }

    pub fn heal(entity: &mut GameObject, amount: f64) -> bool {
        let Some(health) = entity.get_component_mut("Health") else {
            return false;
        };
        health.heal(amount);
        true
    }

    pub fn health(entity: &GameObject) -> Option<f64> {
        entity
            .get_component("Health")
            .map(|health| health.get_f64("health", 0.0))
    }

    pub fn add_item(entity: &mut GameObject, item_id: &str, quantity: i64) -> i64 {
        if entity.get_component("Inventory").is_none() {
            entity.add_component(default_component("Inventory").expect("Inventory"));
        }
        entity
            .get_component_mut("Inventory")
            .map(|inventory| inventory.inventory_add_item(item_id, quantity, json!({})))
            .unwrap_or(0)
    }

    pub fn remove_item(entity: &mut GameObject, item_id: &str, quantity: i64) -> i64 {
        entity
            .get_component_mut("Inventory")
            .map(|inventory| inventory.inventory_remove_item(item_id, quantity))
            .unwrap_or(0)
    }

    pub fn item_count(entity: &GameObject, item_id: &str) -> i64 {
        entity
            .get_component("Inventory")
            .map(|inventory| inventory.inventory_count_item(item_id))
            .unwrap_or(0)
    }

    pub fn add_resource(entity: &mut GameObject, resource_type: &str, amount: f64) -> Option<f64> {
        if entity.get_component("EconomyWallet").is_none() {
            entity.add_component(default_component("EconomyWallet").expect("EconomyWallet"));
        }
        entity
            .get_component_mut("EconomyWallet")
            .map(|wallet| wallet.economy_add(resource_type, amount))
    }

    pub fn spend_resource(entity: &mut GameObject, resource_type: &str, amount: f64) -> bool {
        entity
            .get_component_mut("EconomyWallet")
            .map(|wallet| wallet.economy_spend(resource_type, amount))
            .unwrap_or(false)
    }

    pub fn make_rts_unit(entity: &mut GameObject, team_id: i64, worker: bool) {
        entity.tag = if team_id == 1 { "Player" } else { "Enemy" }.to_string();
        entity.layer = "Units".to_string();
        Self::add_component(entity, "Team", Some(json!({"team_id": team_id})));
        Self::add_component(entity, "Commandable", None);
        Self::add_component(entity, "Vision", None);
        Self::add_component(entity, "NavAgent", None);
        Self::add_component(entity, "SquadMember", None);
        Self::add_component(entity, "Blackboard", None);
        if worker {
            Self::add_component(entity, "Worker", None);
            if let Some(commandable) = entity.get_component_mut("Commandable") {
                commandable.set("can_gather", json!(true));
                commandable.set("can_build", json!(true));
            }
        }
        entity.sync_to_components();
    }

    pub fn enqueue_production(
        producer: &mut GameObject,
        unit_type: &str,
        display_name: &str,
        build_time: f64,
        cost: Value,
    ) -> bool {
        RTSSystem::enqueue_production(producer, unit_type, display_name, build_time, cost)
    }

    pub fn set_blackboard(entity: &mut GameObject, key: &str, value: Value) -> bool {
        if entity.get_component("Blackboard").is_none() {
            entity.add_component(default_component("Blackboard").expect("Blackboard"));
        }
        if let Some(blackboard) = entity.get_component_mut("Blackboard") {
            blackboard.blackboard_set(key, value);
            true
        } else {
            false
        }
    }

    pub fn get_blackboard(entity: &GameObject, key: &str, default: Value) -> Value {
        entity
            .get_component("Blackboard")
            .map(|blackboard| blackboard.blackboard_get(key, default.clone()))
            .unwrap_or(default)
    }

    pub fn start_cooldown(entity: &mut GameObject, name: &str, duration: f64) {
        if entity.get_component("Cooldown").is_none() {
            entity.add_component(default_component("Cooldown").expect("Cooldown"));
        }
        if let Some(cooldown) = entity.get_component_mut("Cooldown") {
            cooldown.cooldown_start(name, duration);
        }
    }

    pub fn cooldown_ready(entity: &GameObject, name: &str) -> bool {
        entity
            .get_component("Cooldown")
            .map(|cooldown| cooldown.cooldown_ready(name))
            .unwrap_or(true)
    }

    pub fn add_status_effect(
        entity: &mut GameObject,
        name: &str,
        duration: f64,
        stacks: i64,
        data: Value,
    ) -> bool {
        if entity.get_component("StatusEffects").is_none() {
            entity.add_component(default_component("StatusEffects").expect("StatusEffects"));
        }
        if let Some(status) = entity.get_component_mut("StatusEffects") {
            status.status_add_effect(name, duration, stacks, data);
            true
        } else {
            false
        }
    }

    pub fn tween(entity: &mut GameObject, property_path: &str, to_value: f64, duration: f64) {
        if entity.get_component("Tween").is_none() {
            entity.add_component(default_component("Tween").expect("Tween"));
        }
        let from_value = Self::read_property_path(entity, property_path, 0.0);
        if let Some(tween) = entity.get_component_mut("Tween") {
            tween.set("property_path", json!(property_path));
            tween.set_f64("from_value", from_value);
            tween.set_f64("to_value", to_value);
            tween.set_f64("duration", duration.max(0.0));
            tween.set_f64("elapsed", 0.0);
            tween.set("easing", json!("smooth"));
            tween.set("active", json!(true));
        }
    }

    pub fn read_property_path(entity: &GameObject, property_path: &str, default: f64) -> f64 {
        if let Some((component_type, attr)) = property_path.split_once('.') {
            return entity
                .get_component(component_type)
                .map(|component| component.get_f64(attr, default))
                .unwrap_or(default);
        }
        match property_path {
            "x" => entity.x,
            "y" => entity.y,
            "rotation" => entity.rotation,
            "scale_x" => entity.scale_x,
            "scale_y" => entity.scale_y,
            "width" => entity.width,
            "height" => entity.height,
            _ => default,
        }
    }

    pub fn add_quest(
        entity: &mut GameObject,
        quest_id: &str,
        title: &str,
        objectives: Value,
    ) -> bool {
        if entity.get_component("QuestLog").is_none() {
            entity.add_component(default_component("QuestLog").expect("QuestLog"));
        }
        entity
            .get_component_mut("QuestLog")
            .map(|quest_log| quest_log.quest_add(quest_id, title, objectives))
            .unwrap_or(false)
    }

    pub fn complete_quest(entity: &mut GameObject, quest_id: &str) -> bool {
        entity
            .get_component_mut("QuestLog")
            .map(|quest_log| quest_log.quest_complete(quest_id))
            .unwrap_or(false)
    }

    pub fn save_game_state(entities: &mut [GameObject], path: impl AsRef<Path>) -> io::Result<()> {
        let serializable = entities
            .iter_mut()
            .filter(|entity| entity.get_component("Saveable").is_some())
            .map(GameObject::serialize)
            .collect::<Vec<_>>();
        AssetTools::write_json(path, &json!({"entities": serializable}))
    }

    pub fn load_game_state(
        entities: &mut [GameObject],
        path: impl AsRef<Path>,
    ) -> io::Result<bool> {
        if !path.as_ref().exists() {
            return Ok(false);
        }
        let data =
            serde_json::from_str::<Value>(&fs::read_to_string(path)?).map_err(io::Error::other)?;
        let Some(saved_entities) = data.get("entities").and_then(Value::as_array) else {
            return Ok(false);
        };

        for saved in saved_entities {
            let save_key =
                saved
                    .get("components")
                    .and_then(Value::as_array)
                    .and_then(|components| {
                        components.iter().find_map(|component| {
                            if component.get("component_type").and_then(Value::as_str)
                                == Some("Saveable")
                            {
                                component.get("save_key").and_then(Value::as_str)
                            } else {
                                None
                            }
                        })
                    });
            let Some(save_key) = save_key else {
                continue;
            };
            let Some(entity) = entities.iter_mut().find(|entity| {
                entity
                    .get_component("Saveable")
                    .and_then(|saveable| saveable.get("save_key"))
                    .and_then(Value::as_str)
                    == Some(save_key)
            }) else {
                continue;
            };
            entity.x = saved.get("x").and_then(Value::as_f64).unwrap_or(entity.x);
            entity.y = saved.get("y").and_then(Value::as_f64).unwrap_or(entity.y);
            entity.sync_to_components();
        }
        Ok(true)
    }
}
