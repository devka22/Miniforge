use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::component::{Component, component_from_data, default_component};
use crate::engine::entity_id::{
    generate_entity_id, generate_entity_name, register_existing_entity_id, register_existing_name,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GameObject {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub id: u64,
    pub name: String,
    pub enabled: bool,
    pub active: bool,
    pub visible: bool,
    pub locked: bool,
    pub selected: bool,
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub width: f64,
    pub height: f64,
    pub speed: f64,
    pub radius: f64,
    pub sprite_name: Option<String>,
    pub sprite_guid: Option<String>,
    pub script: Option<String>,
    pub tag: String,
    pub layer: String,
    #[serde(default)]
    pub editor_group: Option<String>,
    pub parent_id: Option<u64>,
    pub local_x: f64,
    pub local_y: f64,
    pub prefab_source: Option<String>,
    pub prefab_guid: Option<String>,
    pub is_prefab_instance: bool,
    pub scene_name: Option<String>,
    pub state: String,
    pub command: String,
    pub path: Vec<(f64, f64)>,
    pub patrol_points: Vec<(f64, f64)>,
    pub patrol_index: usize,
    pub follow_target_id: Option<u64>,
    pub guard_target_id: Option<u64>,
    pub attack_move_target: Option<(f64, f64)>,
    pub gather_target_id: Option<u64>,
    pub components: Vec<Component>,
    #[serde(default)]
    pub scripts: Vec<Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComponentBundleReport {
    pub added: Vec<String>,
    pub existing: Vec<String>,
    pub missing: Vec<String>,
}

impl GameObject {
    pub fn new(x: f64, y: f64, name: Option<String>) -> Self {
        let mut object = Self::bare("GameObject", x, y, name);
        object.add_component(default_component("Transform").expect("Transform component"));
        object.add_component(default_component("Selectable").expect("Selectable component"));
        object
            .add_component(default_component("SpriteRenderer").expect("SpriteRenderer component"));
        object.add_component(default_component("Collider2D").expect("Collider2D component"));
        object.sync_to_components();
        object
    }

    pub fn new_unit(x: f64, y: f64, name: Option<String>) -> Self {
        let mut unit = Self::bare("Unit", x, y, name);
        unit.speed = 4.5;
        unit.add_component(default_component("Transform").expect("Transform component"));
        unit.add_component(default_component("Selectable").expect("Selectable component"));
        unit.add_component(default_component("RTSMovement").expect("RTSMovement component"));
        unit.add_component(default_component("SpriteRenderer").expect("SpriteRenderer component"));
        unit.add_component(default_component("Collider2D").expect("Collider2D component"));
        unit.sync_to_components();
        unit
    }

    fn bare(entity_type: &str, x: f64, y: f64, name: Option<String>) -> Self {
        Self {
            entity_type: entity_type.to_string(),
            id: generate_entity_id(),
            name: name.unwrap_or_else(|| generate_entity_name(entity_type)),
            enabled: true,
            active: true,
            visible: true,
            locked: false,
            selected: false,
            x,
            y,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            width: 1.0,
            height: 1.0,
            speed: 3.5,
            radius: 0.45,
            sprite_name: None,
            sprite_guid: None,
            script: None,
            tag: "Untagged".to_string(),
            layer: "Default".to_string(),
            editor_group: None,
            parent_id: None,
            local_x: 0.0,
            local_y: 0.0,
            prefab_source: None,
            prefab_guid: None,
            is_prefab_instance: false,
            scene_name: None,
            state: "IDLE".to_string(),
            command: "IDLE".to_string(),
            path: Vec::new(),
            patrol_points: Vec::new(),
            patrol_index: 0,
            follow_target_id: None,
            guard_target_id: None,
            attack_move_target: None,
            gather_target_id: None,
            components: Vec::new(),
            scripts: Vec::new(),
        }
    }

    pub fn add_component(&mut self, component: Component) -> &mut Component {
        if let Some(index) = self
            .components
            .iter()
            .position(|existing| existing.component_type == component.component_type)
        {
            return &mut self.components[index];
        }
        self.components.push(component);
        self.components.last_mut().expect("component just pushed")
    }

    pub fn ensure_components(&mut self, component_types: &[&str]) -> ComponentBundleReport {
        let mut report = ComponentBundleReport::default();
        for component_type in component_types {
            if self.get_component(component_type).is_some() {
                report.existing.push((*component_type).to_string());
                continue;
            }
            if let Some(component) = default_component(component_type) {
                self.add_component(component);
                report.added.push((*component_type).to_string());
            } else {
                report.missing.push((*component_type).to_string());
            }
        }
        report
    }

    pub fn component_types(&self) -> Vec<String> {
        self.components
            .iter()
            .map(|component| component.component_type.clone())
            .collect()
    }

    pub fn is_runtime_active(&self) -> bool {
        self.enabled && self.active
    }

    pub fn get_component(&self, component_type: &str) -> Option<&Component> {
        self.components
            .iter()
            .find(|component| component.component_type == component_type)
    }

    pub fn get_component_mut(&mut self, component_type: &str) -> Option<&mut Component> {
        self.components
            .iter_mut()
            .find(|component| component.component_type == component_type)
    }

    pub fn remove_component(&mut self, component_type: &str) {
        self.components
            .retain(|component| component.component_type != component_type);
    }

    pub fn set_selected(&mut self, value: bool) {
        self.selected = value;
    }

    pub fn sync_from_components(&mut self) {
        if let Some(transform) = self.get_component("Transform") {
            let x = transform.get_f64("x", self.x);
            let y = transform.get_f64("y", self.y);
            let rotation = transform.get_f64("rotation", self.rotation);
            let scale_x = transform.get_f64("scale_x", self.scale_x);
            let scale_y = transform.get_f64("scale_y", self.scale_y);
            self.x = x;
            self.y = y;
            self.rotation = rotation;
            self.scale_x = scale_x;
            self.scale_y = scale_y;
        }
        if let Some(movement) = self.get_component("RTSMovement") {
            let speed = movement.get_f64("speed", self.speed);
            self.speed = speed;
        }
        if let Some(sprite) = self.get_component("SpriteRenderer") {
            let sprite_name = sprite
                .get("sprite_name")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let sprite_guid = sprite
                .get("sprite_guid")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            self.sprite_name = sprite_name;
            self.sprite_guid = sprite_guid;
        }
        if let Some(collider) = self.get_component("Collider2D") {
            let radius = collider.get_f64("radius", self.radius);
            let width = collider.get_f64("width", self.width);
            let height = collider.get_f64("height", self.height);
            self.radius = radius;
            self.width = width;
            self.height = height;
        }
    }

    pub fn sync_to_components(&mut self) {
        self.sync_runtime_motion_to_components();

        let sprite_name = self.sprite_name.clone();
        let sprite_guid = self.sprite_guid.clone();
        if let Some(sprite) = self.get_component_mut("SpriteRenderer") {
            sprite.set("sprite_name", json!(sprite_name));
            sprite.set("sprite_guid", json!(sprite_guid));
        }

        let (radius, width, height) = (self.radius, self.width, self.height);
        if let Some(collider) = self.get_component_mut("Collider2D") {
            collider.set_f64("radius", radius);
            collider.set_f64("width", width);
            collider.set_f64("height", height);
        }
    }

    /// Synchronizes only fields mutated by the per-frame movement pass.
    ///
    /// The full component sync also rewrites sprite and collider JSON values;
    /// doing that for every moving entity every frame creates avoidable string
    /// clones and map churn.
    pub fn sync_runtime_motion_to_components(&mut self) {
        let (x, y, rotation, scale_x, scale_y) =
            (self.x, self.y, self.rotation, self.scale_x, self.scale_y);
        if let Some(transform) = self.get_component_mut("Transform") {
            transform.set_f64("x", x);
            transform.set_f64("y", y);
            transform.set_f64("rotation", rotation);
            transform.set_f64("scale_x", scale_x);
            transform.set_f64("scale_y", scale_y);
        }

        let speed = self.speed;
        if let Some(movement) = self.get_component_mut("RTSMovement") {
            movement.set_f64("speed", speed);
        }
    }

    pub fn update_movement(&mut self, dt: f64) {
        if self.command == "HOLD" {
            self.path.clear();
            self.set_runtime_state("HOLD");
            return;
        }

        let Some(&(target_x, target_y)) = self.path.first() else {
            if !["HOLD", "STOP", "FOLLOW", "GUARD", "GATHER"].contains(&self.command.as_str()) {
                self.set_runtime_state("IDLE");
            }
            return;
        };

        self.set_runtime_state("MOVING");
        let dx = target_x - self.x;
        let dy = target_y - self.y;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance < 0.04 {
            self.x = target_x;
            self.y = target_y;
            self.path.remove(0);
            return;
        }

        if distance > 0.0 {
            let step = (self.speed * dt).min(distance);
            let smoothing = (dt * 12.0).min(1.0);
            let target_step_x = self.x + (dx / distance) * step;
            let target_step_y = self.y + (dy / distance) * step;
            self.x += (target_step_x - self.x) * smoothing;
            self.y += (target_step_y - self.y) * smoothing;
        }
    }

    pub fn set_runtime_state(&mut self, state: &str) {
        if self.state != state {
            self.state.clear();
            self.state.push_str(state);
        }
    }

    pub fn serialize(&mut self) -> Value {
        self.sync_from_components();
        json!({
            "type": self.entity_type,
            "id": self.id,
            "name": self.name,
            "enabled": self.enabled,
            "active": self.enabled,
            "visible": self.visible,
            "locked": self.locked,
            "x": self.x,
            "y": self.y,
            "position": [self.x, self.y],
            "rotation": self.rotation,
            "scale": [self.scale_x, self.scale_y],
            "scale_x": self.scale_x,
            "scale_y": self.scale_y,
            "size": [self.width, self.height],
            "width": self.width,
            "height": self.height,
            "speed": self.speed,
            "radius": self.radius,
            "sprite_name": self.sprite_name,
            "sprite_guid": self.sprite_guid,
            "script": self.script,
            "tag": self.tag,
            "layer": self.layer,
            "editor_group": self.editor_group,
            "state": self.state,
            "command": self.command,
            "path": self.path,
            "parent_id": self.parent_id,
            "local_x": self.local_x,
            "local_y": self.local_y,
            "prefab_source": self.prefab_source,
            "prefab_guid": self.prefab_guid,
            "is_prefab_instance": self.is_prefab_instance,
            "scene_name": self.scene_name,
            "patrol_points": self.patrol_points,
            "patrol_index": self.patrol_index,
            "follow_target_id": self.follow_target_id,
            "guard_target_id": self.guard_target_id,
            "attack_move_target": self.attack_move_target,
            "gather_target_id": self.gather_target_id,
            "components": self.components.iter().map(Component::serialize).collect::<Vec<_>>(),
            "scripts": self.scripts,
        })
    }

    pub fn from_data(data: &Value, preserve_id: bool) -> Self {
        let entity_type = data
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("GameObject");
        let mut object = if entity_type == "Unit" {
            Self::new_unit(
                data.get("x").and_then(Value::as_f64).unwrap_or(0.0),
                data.get("y").and_then(Value::as_f64).unwrap_or(0.0),
                data.get("name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
            )
        } else {
            Self::new(
                data.get("x").and_then(Value::as_f64).unwrap_or(0.0),
                data.get("y").and_then(Value::as_f64).unwrap_or(0.0),
                data.get("name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
            )
        };

        if preserve_id && let Some(id) = data.get("id").and_then(Value::as_u64) {
            object.id = id;
            register_existing_entity_id(id);
        }

        register_existing_name(&object.name);

        if let Some(position) = data.get("position").and_then(Value::as_array)
            && position.len() >= 2
        {
            object.x = position[0].as_f64().unwrap_or(object.x);
            object.y = position[1].as_f64().unwrap_or(object.y);
        }

        object.enabled = data
            .get("enabled")
            .or_else(|| data.get("active"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        object.active = object.enabled;
        object.visible = data.get("visible").and_then(Value::as_bool).unwrap_or(true);
        object.locked = data.get("locked").and_then(Value::as_bool).unwrap_or(false);
        object.rotation = data.get("rotation").and_then(Value::as_f64).unwrap_or(0.0);

        if let Some(scale) = data.get("scale").and_then(Value::as_array) {
            if scale.len() >= 2 {
                object.scale_x = scale[0].as_f64().unwrap_or(1.0);
                object.scale_y = scale[1].as_f64().unwrap_or(1.0);
            }
        } else {
            object.scale_x = data.get("scale_x").and_then(Value::as_f64).unwrap_or(1.0);
            object.scale_y = data.get("scale_y").and_then(Value::as_f64).unwrap_or(1.0);
        }

        if let Some(size) = data.get("size").and_then(Value::as_array) {
            if size.len() >= 2 {
                object.width = size[0].as_f64().unwrap_or(1.0);
                object.height = size[1].as_f64().unwrap_or(1.0);
            }
        } else {
            object.width = data.get("width").and_then(Value::as_f64).unwrap_or(1.0);
            object.height = data.get("height").and_then(Value::as_f64).unwrap_or(1.0);
        }

        object.speed = data
            .get("speed")
            .and_then(Value::as_f64)
            .unwrap_or(object.speed);
        object.radius = data.get("radius").and_then(Value::as_f64).unwrap_or(0.45);
        object.sprite_name = data
            .get("sprite_name")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        object.sprite_guid = data
            .get("sprite_guid")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        object.script = data
            .get("script")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        object.tag = data
            .get("tag")
            .and_then(Value::as_str)
            .unwrap_or("Untagged")
            .to_string();
        object.layer = data
            .get("layer")
            .and_then(Value::as_str)
            .unwrap_or("Default")
            .to_string();
        object.editor_group = data
            .get("editor_group")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        object.parent_id = data.get("parent_id").and_then(Value::as_u64);
        object.local_x = data.get("local_x").and_then(Value::as_f64).unwrap_or(0.0);
        object.local_y = data.get("local_y").and_then(Value::as_f64).unwrap_or(0.0);
        object.prefab_source = data
            .get("prefab_source")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        object.prefab_guid = data
            .get("prefab_guid")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        object.is_prefab_instance = data
            .get("is_prefab_instance")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        object.scene_name = data
            .get("scene_name")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        object.state = data
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("IDLE")
            .to_string();
        object.command = data
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("IDLE")
            .to_string();
        object.path = parse_points(data.get("path"));
        object.patrol_points = parse_points(data.get("patrol_points"));
        object.patrol_index = data
            .get("patrol_index")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;
        object.follow_target_id = data.get("follow_target_id").and_then(Value::as_u64);
        object.guard_target_id = data.get("guard_target_id").and_then(Value::as_u64);
        object.attack_move_target = data.get("attack_move_target").and_then(parse_point);
        object.gather_target_id = data.get("gather_target_id").and_then(Value::as_u64);

        object.components.clear();
        if let Some(components) = data.get("components").and_then(Value::as_array) {
            for component_data in components {
                if let Some(component) = component_from_data(component_data) {
                    object.add_component(component);
                }
            }
        }
        object.scripts = data
            .get("scripts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        object.sync_to_components();
        object
    }
}

pub fn game_object_from_data(data: &Value, preserve_id: bool) -> GameObject {
    GameObject::from_data(data, preserve_id)
}

fn parse_points(value: Option<&Value>) -> Vec<(f64, f64)> {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(parse_point).collect())
        .unwrap_or_default()
}

fn parse_point(value: &Value) -> Option<(f64, f64)> {
    let coords = value.as_array()?;
    if coords.len() < 2 {
        return None;
    }
    Some((coords[0].as_f64()?, coords[1].as_f64()?))
}
