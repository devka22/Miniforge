use std::fs;
use std::io;
use std::path::Path;

use serde_json::{Value, json};

use crate::engine::archetype_library::ArchetypeLibrary;
use crate::engine::asset_tools::AssetTools;
use crate::engine::build_placement::{BuildFootprint, BuildPlacement, PlacementResult};
use crate::engine::component::{Component, component_from_data, default_component};
use crate::engine::spatial_index::{SpatialEntry, SpatialIndex};
use crate::engine::survival_systems::{
    CraftResult, EquipmentChangeResult, EquipmentSummary, InjuryResult, SurvivalEnvironment2D,
    SurvivalInteractionResult, SurvivalSystems, SurvivalTickReport,
};
use crate::engine::survival_world::{
    BarricadeResult, DoorActionResult, DoorCommand, NoiseEvent2D, PerceptionResult,
    SurvivalWorldSystems,
};
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

    pub fn spawn(entities: &mut Vec<GameObject>, name: &str, x: f64, y: f64) -> u64 {
        Self::create_game_object(entities, name, x, y)
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

    pub fn destroy_named(entities: &mut Vec<GameObject>, name: &str) -> usize {
        let before = entities.len();
        entities.retain(|entity| entity.name != name);
        before.saturating_sub(entities.len())
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

    pub fn move_entity(entity: &mut GameObject, dx: f64, dy: f64) {
        Self::translate(entity, dx, dy);
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

    pub fn query_radius_ids_indexed(
        index: &SpatialIndex,
        x: f64,
        y: f64,
        radius: f64,
        tag: Option<&str>,
        layer: Option<&str>,
    ) -> Vec<u64> {
        index.query_radius_ids(x, y, radius, tag, layer)
    }

    pub fn query_radius_ids_indexed_into(
        index: &SpatialIndex,
        x: f64,
        y: f64,
        radius: f64,
        tag: Option<&str>,
        layer: Option<&str>,
        output: &mut Vec<u64>,
    ) {
        index.query_radius_ids_into(x, y, radius, tag, layer, output);
    }

    pub fn any_in_radius_indexed(
        index: &SpatialIndex,
        x: f64,
        y: f64,
        radius: f64,
        tag: Option<&str>,
        layer: Option<&str>,
    ) -> bool {
        index.any_in_radius(x, y, radius, tag, layer)
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

    pub fn play_sound(
        entities: &mut Vec<GameObject>,
        audio_name: &str,
        bus: &str,
        volume: f64,
        looped: bool,
    ) -> u64 {
        let mut entity = GameObject::new(0.0, 0.0, Some(format!("Audio_{audio_name}")));
        entity.visible = false;
        entity.locked = true;
        if let Some(source) = Self::add_component(&mut entity, "AudioSource", None) {
            source.set("audio_name", json!(audio_name));
            source.set("bus", json!(bus));
            source.set_f64("volume", volume.clamp(0.0, 1.0));
            source.set("loop", json!(looped));
            source.set("play_on_start", json!(true));
        }
        let id = entity.id;
        entities.push(entity);
        id
    }

    pub fn load_scene_request(scene_name: &str) -> Value {
        json!({"command": "load_scene", "scene": scene_name})
    }

    pub fn input_pressed(pressed_keys: &[String], key: &str) -> bool {
        pressed_keys.iter().any(|pressed| pressed == key)
    }

    pub fn set_ui_text_by_id(entities: &mut [GameObject], entity_id: u64, text: &str) -> bool {
        let Some(entity) = entities.iter_mut().find(|entity| entity.id == entity_id) else {
            return false;
        };
        Self::set_ui_text(entity, text)
    }

    pub fn set_ui_text_by_name(entities: &mut [GameObject], name: &str, text: &str) -> bool {
        let Some(entity) = entities.iter_mut().find(|entity| entity.name == name) else {
            return false;
        };
        Self::set_ui_text(entity, text)
    }

    pub fn set_ui_text(entity: &mut GameObject, text: &str) -> bool {
        if entity.get_component("UIElement").is_none() {
            let _ = Self::add_component(entity, "UIElement", None);
        }
        let Some(component) = entity.get_component_mut("UIElement") else {
            return false;
        };
        component.set("text", json!(text));
        true
    }

    pub fn set_ui_progress(entity: &mut GameObject, value: f64, max: f64) -> bool {
        if entity.get_component("UIElement").is_none() {
            let _ = Self::add_component(entity, "UIElement", None);
        }
        let Some(component) = entity.get_component_mut("UIElement") else {
            return false;
        };
        component.set("element_type", json!("ProgressBar"));
        component.set_f64("progress", value.max(0.0));
        component.set_f64("max_progress", max.max(0.0001));
        true
    }

    pub fn set_ui_visible(entity: &mut GameObject, visible: bool) -> bool {
        entity.visible = visible;
        if entity.get_component("UIElement").is_none() {
            let _ = Self::add_component(entity, "UIElement", None);
        }
        let Some(component) = entity.get_component_mut("UIElement") else {
            return false;
        };
        component.set("visible", json!(visible));
        true
    }

    pub fn set_tag(entity: &mut GameObject, tag: &str) {
        entity.tag = tag.to_string();
        if let Some(component) = entity.get_component_mut("Actor2D") {
            component.set("tag", json!(tag));
        }
    }

    pub fn set_layer(entity: &mut GameObject, layer: &str) {
        entity.layer = layer.to_string();
        if let Some(component) = entity.get_component_mut("Actor2D") {
            component.set("layer", json!(layer));
        }
    }

    pub fn set_enabled(entity: &mut GameObject, enabled: bool) {
        entity.enabled = enabled;
        entity.active = enabled;
    }

    pub fn set_visible(entity: &mut GameObject, visible: bool) {
        entity.visible = visible;
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

    pub fn has_item(entity: &GameObject, item_id: &str, quantity: i64) -> bool {
        entity
            .get_component("Inventory")
            .map(|inventory| inventory.inventory_has_item(item_id, quantity.max(1)))
            .unwrap_or(false)
    }

    pub fn transfer_item(
        from: &mut GameObject,
        to: &mut GameObject,
        item_id: &str,
        quantity: i64,
    ) -> i64 {
        let removed = Self::remove_item(from, item_id, quantity);
        if removed <= 0 {
            return 0;
        }
        let added = Self::add_item(to, item_id, removed);
        if added < removed {
            let _ = Self::add_item(from, item_id, removed - added);
        }
        added
    }

    pub fn inventory_slots_used(entity: &GameObject) -> usize {
        entity
            .get_component("Inventory")
            .and_then(|inventory| inventory.get("items"))
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0)
    }

    pub fn inventory_space_left(entity: &GameObject) -> i64 {
        entity
            .get_component("Inventory")
            .map(|inventory| {
                let capacity = inventory.get_i64("capacity", 24).max(0);
                capacity - Self::inventory_slots_used(entity) as i64
            })
            .unwrap_or(0)
            .max(0)
    }

    pub fn inventory_weight(entity: &GameObject) -> f64 {
        entity
            .get_component("Inventory")
            .map(Component::inventory_weight)
            .unwrap_or(0.0)
    }

    pub fn sort_inventory(entity: &mut GameObject, mode: &str) -> bool {
        let Some(inventory) = entity.get_component_mut("Inventory") else {
            return false;
        };
        inventory.inventory_sort_items(mode);
        true
    }

    pub fn survival_need(entity: &GameObject, name: &str) -> Option<f64> {
        SurvivalSystems::need(entity, name)
    }

    pub fn survival_state(entity: &GameObject) -> Value {
        SurvivalSystems::state(entity)
    }

    pub fn tick_survival(entity: &mut GameObject, dt: f64) -> SurvivalTickReport {
        SurvivalSystems::tick_entity(entity, dt)
    }

    pub fn tick_survival_in_environment(
        entity: &mut GameObject,
        dt: f64,
        environment: &SurvivalEnvironment2D,
    ) -> SurvivalTickReport {
        SurvivalSystems::tick_entity_in_environment(entity, dt, environment)
    }

    pub fn set_survival_need(entity: &mut GameObject, name: &str, value: f64) -> bool {
        SurvivalSystems::set_need(entity, name, value)
    }

    pub fn modify_survival_need(entity: &mut GameObject, name: &str, delta: f64) -> bool {
        SurvivalSystems::modify_need(entity, name, delta)
    }

    pub fn use_item(entity: &mut GameObject, item_id: &str) -> bool {
        SurvivalSystems::use_item(entity, item_id)
    }

    pub fn search_loot_container(container: &mut GameObject) -> usize {
        SurvivalSystems::search_container(container)
    }

    pub fn rummage_loot_container(container: &mut GameObject) -> usize {
        SurvivalSystems::rummage_container(container)
    }

    pub fn take_container_item(
        container: &mut GameObject,
        actor: &mut GameObject,
        item_id: &str,
        quantity: i64,
    ) -> i64 {
        SurvivalSystems::take_from_container(container, actor, item_id, quantity)
    }

    pub fn take_all_container_items(container: &mut GameObject, actor: &mut GameObject) -> i64 {
        SurvivalSystems::take_all(container, actor)
    }

    pub fn can_craft(entity: &GameObject, recipe_id: &str) -> bool {
        SurvivalSystems::can_craft(entity, recipe_id)
    }

    pub fn craft(entity: &mut GameObject, recipe_id: &str) -> CraftResult {
        SurvivalSystems::craft(entity, recipe_id)
    }

    pub fn craft_at(actor: &mut GameObject, station: &GameObject, recipe_id: &str) -> CraftResult {
        SurvivalSystems::craft_at(actor, station, recipe_id)
    }

    pub fn harvest(source: &mut GameObject, actor: &mut GameObject) -> i64 {
        SurvivalSystems::harvest(source, actor)
    }

    pub fn survival_interact(
        actor: &mut GameObject,
        target: &mut GameObject,
    ) -> SurvivalInteractionResult {
        SurvivalSystems::interact(actor, target)
    }

    pub fn equip_item(entity: &mut GameObject, slot: &str, item_id: &str, bonuses: Value) -> bool {
        if entity.get_component("Equipment").is_none() {
            entity.add_component(default_component("Equipment").expect("Equipment"));
        }
        entity
            .get_component_mut("Equipment")
            .map(|equipment| equipment.equipment_equip(slot, Some(item_id), bonuses))
            .unwrap_or(false)
    }

    pub fn unequip_item(entity: &mut GameObject, slot: &str) -> Option<Value> {
        entity
            .get_component_mut("Equipment")
            .and_then(|equipment| equipment.equipment_unequip(slot))
    }

    pub fn equip_inventory_item(
        entity: &mut GameObject,
        item_id: &str,
        preferred_slot: Option<&str>,
    ) -> EquipmentChangeResult {
        SurvivalSystems::equip_from_inventory(entity, item_id, preferred_slot)
    }

    pub fn unequip_to_inventory(entity: &mut GameObject, slot: &str) -> EquipmentChangeResult {
        SurvivalSystems::unequip_to_inventory(entity, slot)
    }

    pub fn equipment_summary(entity: &GameObject) -> EquipmentSummary {
        SurvivalSystems::equipment_summary(entity)
    }

    pub fn effective_stat(entity: &GameObject, stat: &str) -> f64 {
        SurvivalSystems::effective_stat(entity, stat)
    }

    pub fn degrade_equipment(entity: &mut GameObject, slot: &str, amount: f64) -> Option<f64> {
        SurvivalSystems::degrade_equipped_item(entity, slot, amount)
    }

    pub fn apply_injury(
        entity: &mut GameObject,
        region: &str,
        injury_type: &str,
        severity: f64,
    ) -> InjuryResult {
        SurvivalSystems::apply_injury(entity, region, injury_type, severity)
    }

    pub fn treat_injury(entity: &mut GameObject, injury_id: u64, item_id: &str) -> InjuryResult {
        SurvivalSystems::treat_injury_with_item(entity, injury_id, item_id)
    }

    pub fn set_crouching(entity: &mut GameObject, crouching: bool) -> bool {
        SurvivalWorldSystems::set_crouching(entity, crouching)
    }

    pub fn stealth_movement_multiplier(entity: &GameObject) -> f64 {
        SurvivalWorldSystems::movement_multiplier(entity)
    }

    pub fn stealth_visibility(entity: &GameObject) -> f64 {
        SurvivalWorldSystems::visibility(entity)
    }

    pub fn emit_noise(entity: &mut GameObject, kind: &str, scale: f64) -> NoiseEvent2D {
        SurvivalWorldSystems::emit_noise(entity, kind, scale)
    }

    pub fn tick_noise(entity: &mut GameObject, dt: f64) {
        SurvivalWorldSystems::tick_noise(entity, dt)
    }

    pub fn perceive(
        observer: &mut GameObject,
        candidates: &[GameObject],
        noises: &[NoiseEvent2D],
        dt: f64,
    ) -> PerceptionResult {
        SurvivalWorldSystems::update_perception(observer, candidates, noises, dt)
    }

    pub fn door_action(
        target: &mut GameObject,
        action: &str,
        key_id: Option<&str>,
    ) -> DoorActionResult {
        let command = match action {
            "open" => DoorCommand::Open,
            "close" => DoorCommand::Close,
            "lock" => DoorCommand::Lock,
            "unlock" => DoorCommand::Unlock { key_id },
            _ => DoorCommand::Toggle,
        };
        SurvivalWorldSystems::door_action(target, command)
    }

    pub fn tick_door(target: &mut GameObject, dt: f64) -> bool {
        SurvivalWorldSystems::tick_door(target, dt)
    }

    pub fn add_barricade_layer(target: &mut GameObject) -> BarricadeResult {
        SurvivalWorldSystems::add_barricade_layer(target)
    }

    pub fn barricade_from_inventory(
        actor: &mut GameObject,
        target: &mut GameObject,
    ) -> BarricadeResult {
        let build_item = target
            .get_component("Barricade2D")
            .and_then(|component| component.get("build_item"))
            .and_then(Value::as_str)
            .unwrap_or("wood_plank")
            .to_string();
        let tool_item = target
            .get_component("Barricade2D")
            .and_then(|component| component.get("tool_item"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let item_count = target
            .get_component("Barricade2D")
            .map(|component| component.get_i64("items_per_layer", 1).max(1))
            .unwrap_or(1);
        if !tool_item.is_empty() && !Self::has_item(actor, &tool_item, 1) {
            return BarricadeResult {
                reason: "tool_required".to_string(),
                ..BarricadeResult::default()
            };
        }
        if !Self::has_item(actor, &build_item, item_count) {
            return BarricadeResult {
                reason: "materials_required".to_string(),
                ..BarricadeResult::default()
            };
        }
        let result = SurvivalWorldSystems::add_barricade_layer(target);
        if result.success {
            let _ = Self::remove_item(actor, &build_item, item_count);
        }
        result
    }

    pub fn damage_barricade(target: &mut GameObject, amount: f64) -> BarricadeResult {
        SurvivalWorldSystems::damage_barricade(target, amount)
    }

    pub fn add_resource(entity: &mut GameObject, resource_type: &str, amount: f64) -> Option<f64> {
        if entity.get_component("EconomyWallet").is_none() {
            entity.add_component(default_component("EconomyWallet").expect("EconomyWallet"));
        }
        entity
            .get_component_mut("EconomyWallet")
            .map(|wallet| wallet.economy_add(resource_type, amount))
    }

    pub fn resource_amount(entity: &GameObject, resource_type: &str) -> f64 {
        entity
            .get_component("EconomyWallet")
            .and_then(|wallet| wallet.get("resources"))
            .and_then(Value::as_object)
            .and_then(|resources| resources.get(resource_type))
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    }

    pub fn spend_resource(entity: &mut GameObject, resource_type: &str, amount: f64) -> bool {
        entity
            .get_component_mut("EconomyWallet")
            .map(|wallet| wallet.economy_spend(resource_type, amount))
            .unwrap_or(false)
    }

    pub fn can_afford(entity: &GameObject, cost: &Value) -> bool {
        let Some(costs) = cost.as_object() else {
            return true;
        };
        costs.iter().all(|(resource, amount)| {
            Self::resource_amount(entity, resource) >= amount.as_f64().unwrap_or(0.0).max(0.0)
        })
    }

    pub fn spend_cost(entity: &mut GameObject, cost: &Value) -> bool {
        if !Self::can_afford(entity, cost) {
            return false;
        }
        let Some(costs) = cost.as_object() else {
            return true;
        };
        for (resource, amount) in costs {
            let _ = Self::spend_resource(entity, resource, amount.as_f64().unwrap_or(0.0).max(0.0));
        }
        true
    }

    pub fn add_resources(entity: &mut GameObject, resources: &Value) -> usize {
        let Some(resources) = resources.as_object() else {
            return 0;
        };
        let mut changed = 0;
        for (resource, amount) in resources {
            if Self::add_resource(entity, resource, amount.as_f64().unwrap_or(0.0)).is_some() {
                changed += 1;
            }
        }
        changed
    }

    pub fn transfer_resource(
        from: &mut GameObject,
        to: &mut GameObject,
        resource_type: &str,
        amount: f64,
    ) -> bool {
        if !Self::spend_resource(from, resource_type, amount) {
            return false;
        }
        let _ = Self::add_resource(to, resource_type, amount);
        true
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

    pub fn add_production_recipe(
        producer: &mut GameObject,
        unit_type: &str,
        display_name: &str,
        build_time: f64,
        cost: Value,
    ) -> bool {
        if producer.get_component("ProductionRecipeBook").is_none() {
            producer.add_component(
                default_component("ProductionRecipeBook").expect("ProductionRecipeBook"),
            );
        }
        let Some(book) = producer.get_component_mut("ProductionRecipeBook") else {
            return false;
        };
        let mut recipes = book
            .get("recipes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if let Some(existing) = recipes
            .iter_mut()
            .find(|recipe| recipe.get("unit_type").and_then(Value::as_str) == Some(unit_type))
            && let Some(map) = existing.as_object_mut()
        {
            map.insert("display_name".to_string(), json!(display_name));
            map.insert("build_time".to_string(), json!(build_time.max(0.1)));
            map.insert("cost".to_string(), cost);
            book.set("recipes", Value::Array(recipes));
            return true;
        }
        recipes.push(json!({
            "unit_type": unit_type,
            "display_name": display_name,
            "build_time": build_time.max(0.1),
            "cost": cost,
        }));
        book.set("recipes", Value::Array(recipes));
        true
    }

    pub fn set_preferred_recipe(producer: &mut GameObject, unit_type: &str) -> bool {
        if producer.get_component("ProductionRecipeBook").is_none() {
            producer.add_component(
                default_component("ProductionRecipeBook").expect("ProductionRecipeBook"),
            );
        }
        let Some(book) = producer.get_component_mut("ProductionRecipeBook") else {
            return false;
        };
        book.set("preferred_recipe", json!(unit_type));
        true
    }

    pub fn enqueue_preferred_recipe(producer: &mut GameObject) -> bool {
        if producer.get_component("ProductionQueue").is_none() {
            producer.add_component(default_component("ProductionQueue").expect("ProductionQueue"));
        }
        let Some(book) = producer.get_component("ProductionRecipeBook").cloned() else {
            return false;
        };
        let preferred = book.get_string("preferred_recipe", "Worker");
        let Some(recipe) = book
            .get("recipes")
            .and_then(Value::as_array)
            .and_then(|recipes| {
                recipes
                    .iter()
                    .find(|recipe| {
                        recipe.get("unit_type").and_then(Value::as_str) == Some(preferred.as_str())
                    })
                    .or_else(|| recipes.first())
            })
        else {
            return false;
        };
        Self::enqueue_production(
            producer,
            recipe
                .get("unit_type")
                .and_then(Value::as_str)
                .unwrap_or("Worker"),
            recipe
                .get("display_name")
                .and_then(Value::as_str)
                .unwrap_or("Worker"),
            recipe
                .get("build_time")
                .and_then(Value::as_f64)
                .unwrap_or(3.0),
            recipe.get("cost").cloned().unwrap_or_else(|| json!({})),
        )
    }

    pub fn gather_resource(worker: &mut GameObject, node: &mut GameObject, amount: f64) -> f64 {
        let Some(resource_node) = node.get_component_mut("ResourceNode") else {
            return 0.0;
        };
        let resource_type = resource_node.get_string("resource_type", "Gold");
        let gathered = resource_node.gather(amount.max(0.0));
        if gathered <= 0.0 {
            return 0.0;
        }
        if worker.get_component("Worker").is_none() {
            worker.add_component(default_component("Worker").expect("Worker"));
        }
        worker
            .get_component_mut("Worker")
            .map(|worker| worker.worker_add_resource(&resource_type, gathered))
            .unwrap_or(0.0)
    }

    pub fn deposit_worker_cargo(worker: &mut GameObject, wallet_owner: &mut GameObject) -> f64 {
        let Some(worker_component) = worker.get_component_mut("Worker") else {
            return 0.0;
        };
        let resource_type = worker_component.get_string("carrying_type", "Gold");
        let amount = worker_component.get_f64("carrying_amount", 0.0);
        if amount <= 0.0 {
            return 0.0;
        }
        worker_component.set_f64("carrying_amount", 0.0);
        worker_component.set("carrying_type", Value::Null);
        let _ = Self::add_resource(wallet_owner, &resource_type, amount);
        amount
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

    pub fn trigger_ability(entity: &mut GameObject, now: f64) -> bool {
        if entity.get_component("Ability").is_none() {
            entity.add_component(default_component("Ability").expect("Ability"));
        }
        entity
            .get_component_mut("Ability")
            .map(|ability| ability.ability_trigger(now))
            .unwrap_or(false)
    }

    pub fn recharge_ability(entity: &mut GameObject, amount: i64) -> bool {
        let Some(ability) = entity.get_component_mut("Ability") else {
            return false;
        };
        ability.ability_recharge(amount);
        true
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

    pub fn set_quest_objective_progress(
        entity: &mut GameObject,
        quest_id: &str,
        objective_id: &str,
        progress: Value,
    ) -> bool {
        entity
            .get_component_mut("QuestLog")
            .map(|quest_log| {
                quest_log.quest_set_objective_progress(quest_id, objective_id, progress)
            })
            .unwrap_or(false)
    }

    pub fn save_game_state(entities: &mut [GameObject], path: impl AsRef<Path>) -> io::Result<()> {
        let serializable = entities
            .iter_mut()
            .filter(|entity| entity.get_component("Saveable").is_some())
            .map(GameObject::serialize)
            .collect::<Vec<_>>();
        AssetTools::write_json(
            path,
            &json!({
                "kind": "MiniForgeSaveGame",
                "version": 2,
                "entities": serializable,
            }),
        )
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
            let save_key = save_key_from_data(saved);
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
            let current_id = entity.id;
            let current_scene = entity.scene_name.clone();
            let mut restored = GameObject::from_data(saved, true);
            restored.id = current_id;
            restored.scene_name = restored.scene_name.or(current_scene);
            restored.sync_to_components();
            *entity = restored;
        }
        Ok(true)
    }

    pub fn load_game_state_into(
        entities: &mut Vec<GameObject>,
        path: impl AsRef<Path>,
    ) -> io::Result<usize> {
        if !path.as_ref().exists() {
            return Ok(0);
        }
        let data =
            serde_json::from_str::<Value>(&fs::read_to_string(path)?).map_err(io::Error::other)?;
        let Some(saved_entities) = data.get("entities").and_then(Value::as_array) else {
            return Ok(0);
        };

        let mut restored_count = 0;
        for saved in saved_entities {
            let Some(save_key) = save_key_from_data(saved) else {
                continue;
            };
            if let Some(index) = entities.iter().position(|entity| {
                entity
                    .get_component("Saveable")
                    .and_then(|saveable| saveable.get("save_key"))
                    .and_then(Value::as_str)
                    == Some(save_key)
            }) {
                let current_id = entities[index].id;
                let current_scene = entities[index].scene_name.clone();
                let mut restored = GameObject::from_data(saved, true);
                restored.id = current_id;
                restored.scene_name = restored.scene_name.or(current_scene);
                restored.sync_to_components();
                entities[index] = restored;
                restored_count += 1;
                continue;
            }

            if saved
                .get("components")
                .and_then(Value::as_array)
                .and_then(|components| {
                    components
                        .iter()
                        .find(|component| {
                            component.get("component_type").and_then(Value::as_str)
                                == Some("Saveable")
                        })
                        .and_then(|component| component.get("persistent"))
                        .and_then(Value::as_bool)
                })
                .unwrap_or(false)
            {
                let mut restored = GameObject::from_data(saved, true);
                restored.sync_to_components();
                entities.push(restored);
                restored_count += 1;
            }
        }
        Ok(restored_count)
    }
}

fn save_key_from_data(saved: &Value) -> Option<&str> {
    saved
        .get("components")
        .and_then(Value::as_array)
        .and_then(|components| {
            components.iter().find_map(|component| {
                if component.get("component_type").and_then(Value::as_str) == Some("Saveable") {
                    component.get("save_key").and_then(Value::as_str)
                } else {
                    None
                }
            })
        })
}
