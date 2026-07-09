use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

fn default_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Component {
    #[serde(rename = "component_type")]
    pub component_type: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(flatten)]
    pub data: BTreeMap<String, Value>,
}

impl Component {
    pub fn new(component_type: impl Into<String>) -> Self {
        Self {
            component_type: component_type.into(),
            enabled: true,
            data: BTreeMap::new(),
        }
    }

    pub fn with_defaults(component_type: impl Into<String>, defaults: Value) -> Self {
        let mut component = Self::new(component_type);
        if let Value::Object(map) = defaults {
            for (key, value) in map {
                if key == "component_type" {
                    continue;
                }
                if key == "enabled" {
                    component.enabled = value.as_bool().unwrap_or(true);
                    continue;
                }
                component.data.insert(key, value);
            }
        }
        component
    }

    pub fn serialize(&self) -> Value {
        let mut map = Map::new();
        map.insert(
            "component_type".to_string(),
            Value::String(self.component_type.clone()),
        );
        map.insert("enabled".to_string(), Value::Bool(self.enabled));
        for (key, value) in &self.data {
            if !key.starts_with('_') {
                map.insert(key.clone(), persistent_value(value));
            }
        }
        Value::Object(map)
    }

    pub fn merge_data(&mut self, data: &Value) {
        if let Value::Object(map) = data {
            for (key, value) in map {
                match key.as_str() {
                    "component_type" => {}
                    "enabled" => self.enabled = value.as_bool().unwrap_or(self.enabled),
                    _ => {
                        self.data.insert(key.clone(), value.clone());
                    }
                }
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.data.insert(key.into(), value);
    }

    pub fn get_f64(&self, key: &str, default: f64) -> f64 {
        self.data
            .get(key)
            .and_then(Value::as_f64)
            .unwrap_or(default)
    }

    pub fn set_f64(&mut self, key: &str, value: f64) {
        self.set(key, json!(value));
    }

    pub fn get_i64(&self, key: &str, default: i64) -> i64 {
        self.data
            .get(key)
            .and_then(Value::as_i64)
            .unwrap_or(default)
    }

    pub fn get_usize(&self, key: &str, default: usize) -> usize {
        self.data
            .get(key)
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(default)
    }

    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.data
            .get(key)
            .and_then(Value::as_bool)
            .unwrap_or(default)
    }

    pub fn get_string(&self, key: &str, default: &str) -> String {
        self.data
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or(default)
            .to_string()
    }

    pub fn get_string_list(&self, key: &str) -> Vec<String> {
        self.data
            .get(key)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|value| value.as_str().map(ToString::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn add_force(&mut self, force_x: f64, force_y: f64, impulse: bool) {
        if self.component_type != "Rigidbody2D" {
            return;
        }
        let mass = self.get_f64("mass", 1.0).max(0.0001);
        let scale = if impulse { 1.0 } else { 1.0 / mass };
        let vx = self.get_f64("velocity_x", 0.0) + force_x * scale;
        let vy = self.get_f64("velocity_y", 0.0) + force_y * scale;
        self.set_f64("velocity_x", vx);
        self.set_f64("velocity_y", vy);
        self.set("sleeping", json!(false));
    }

    pub fn is_dynamic_body(&self) -> bool {
        self.component_type == "Rigidbody2D"
            && self.get_string("body_type", "dynamic") == "dynamic"
            && !self.get_bool("sleeping", false)
    }

    pub fn take_damage(&mut self, amount: f64) {
        if self.component_type != "Health" || !self.get_bool("alive", true) {
            return;
        }
        let damage = (amount - self.get_f64("armor", 0.0)).max(0.0);
        let health = (self.get_f64("health", self.get_f64("max_health", 100.0)) - damage).max(0.0);
        self.set_f64("health", health);
        if health <= 0.0 {
            self.set("alive", json!(false));
        }
    }

    pub fn heal(&mut self, amount: f64) {
        if self.component_type != "Health" || !self.get_bool("alive", true) {
            return;
        }
        let max_health = self.get_f64("max_health", 100.0);
        let health = (self.get_f64("health", max_health) + amount).min(max_health);
        self.set_f64("health", health);
    }

    pub fn stats_add_experience(&mut self, amount: f64) -> i64 {
        if self.component_type != "Stats" {
            return 0;
        }
        let mut experience = self.get_f64("experience", 0.0) + amount.max(0.0);
        let mut level = self.get_i64("level", 1);
        let mut next = self.get_f64("experience_to_next", 100.0);
        let mut levels = 0;
        while next > 0.0 && experience >= next {
            experience -= next;
            level += 1;
            levels += 1;
            next = (next * 1.25 + 25.0) * 100.0;
            next = next.round() / 100.0;
        }
        self.set_f64("experience", experience);
        self.set("level", json!(level));
        self.set_f64("experience_to_next", next);
        levels
    }

    pub fn stats_effective_attack(&self) -> f64 {
        if self.component_type != "Stats" {
            return 0.0;
        }
        self.get_f64("attack", 10.0) + self.get_f64("strength", 5.0) * 0.5
    }

    pub fn stats_effective_defense(&self) -> f64 {
        if self.component_type != "Stats" {
            return 0.0;
        }
        self.get_f64("defense", 0.0) + self.get_f64("vitality", 5.0) * 0.25
    }

    pub fn gather(&mut self, amount: f64) -> f64 {
        if self.component_type != "ResourceNode" {
            return 0.0;
        }
        let current = self.get_f64("amount", 0.0);
        let gathered = current.min(amount.max(0.0));
        self.set_f64("amount", current - gathered);
        gathered
    }

    pub fn is_depleted(&self) -> bool {
        self.component_type == "ResourceNode" && self.get_f64("amount", 0.0) <= 0.0
    }

    pub fn worker_add_resource(&mut self, resource_type: &str, amount: f64) -> f64 {
        if self.component_type != "Worker" {
            return 0.0;
        }
        let carrying_type = self
            .data
            .get("carrying_type")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        if let Some(current_type) = carrying_type {
            if current_type != resource_type {
                return 0.0;
            }
        } else {
            self.set("carrying_type", json!(resource_type));
        }
        let capacity = self.get_f64("carry_capacity", 50.0);
        let current = self.get_f64("carrying_amount", 0.0);
        let added = (capacity - current).max(0.0).min(amount.max(0.0));
        self.set_f64("carrying_amount", current + added);
        added
    }

    pub fn inventory_add_item(&mut self, item_id: &str, quantity: i64, metadata: Value) -> i64 {
        if self.component_type != "Inventory" || self.get_bool("locked", false) {
            return 0;
        }

        let mut quantity = quantity.max(0);
        let mut added = 0;
        let capacity = self.get_i64("capacity", 24).max(0) as usize;
        let stack_limit = self.get_i64("stack_limit", 99).max(1);
        let mut items = self
            .data
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        for item in &mut items {
            if quantity <= 0 {
                break;
            }
            let Some(map) = item.as_object_mut() else {
                continue;
            };
            if map.get("id").and_then(Value::as_str) != Some(item_id) {
                continue;
            }
            if !map
                .get("stackable")
                .and_then(Value::as_bool)
                .unwrap_or(true)
            {
                continue;
            }
            let item_limit = map
                .get("stack_limit")
                .and_then(Value::as_i64)
                .unwrap_or(stack_limit)
                .max(1);
            let current = map.get("quantity").and_then(Value::as_i64).unwrap_or(0);
            let moved = (item_limit - current).max(0).min(quantity);
            map.insert("quantity".to_string(), json!(current + moved));
            quantity -= moved;
            added += moved;
        }

        while quantity > 0 && items.len() < capacity {
            let moved = quantity.min(stack_limit);
            items.push(json!({
                "id": item_id,
                "quantity": moved,
                "stackable": true,
                "stack_limit": stack_limit,
                "metadata": metadata,
            }));
            quantity -= moved;
            added += moved;
        }

        self.set("items", Value::Array(items));
        added
    }

    pub fn inventory_remove_item(&mut self, item_id: &str, quantity: i64) -> i64 {
        if self.component_type != "Inventory" {
            return 0;
        }
        let mut quantity = quantity.max(0);
        let mut removed = 0;
        let mut retained = Vec::new();
        let items = self
            .data
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        for mut item in items {
            let matches = item.get("id").and_then(Value::as_str) == Some(item_id);
            if matches && quantity > 0 {
                let current = item.get("quantity").and_then(Value::as_i64).unwrap_or(0);
                let moved = current.min(quantity);
                quantity -= moved;
                removed += moved;
                let remaining = current - moved;
                if remaining > 0 {
                    if let Some(map) = item.as_object_mut() {
                        map.insert("quantity".to_string(), json!(remaining));
                    }
                    retained.push(item);
                }
            } else {
                retained.push(item);
            }
        }
        self.set("items", Value::Array(retained));
        removed
    }

    pub fn inventory_count_item(&self, item_id: &str) -> i64 {
        if self.component_type != "Inventory" {
            return 0;
        }
        self.data
            .get("items")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter(|item| item.get("id").and_then(Value::as_str) == Some(item_id))
                    .map(|item| item.get("quantity").and_then(Value::as_i64).unwrap_or(0))
                    .sum()
            })
            .unwrap_or(0)
    }

    pub fn inventory_has_item(&self, item_id: &str, quantity: i64) -> bool {
        self.inventory_count_item(item_id) >= quantity
    }

    pub fn equipment_equip(&mut self, slot: &str, item_id: Option<&str>, bonuses: Value) -> bool {
        if self.component_type != "Equipment" {
            return false;
        }
        let locked = self.get_string_list("locked_slots");
        if locked.iter().any(|locked_slot| locked_slot == slot) {
            return false;
        }
        let mut slots = self
            .data
            .get("slots")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        slots.insert(
            slot.to_string(),
            item_id.map_or(Value::Null, |id| json!(id)),
        );
        self.set("slots", Value::Object(slots));

        let mut stat_bonuses = self
            .data
            .get("stat_bonuses")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        stat_bonuses.insert(slot.to_string(), bonuses);
        self.set("stat_bonuses", Value::Object(stat_bonuses));
        true
    }

    pub fn equipment_unequip(&mut self, slot: &str) -> Option<Value> {
        if self.component_type != "Equipment" {
            return None;
        }
        let mut slots = self
            .data
            .get("slots")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let previous = slots.insert(slot.to_string(), Value::Null);
        self.set("slots", Value::Object(slots));
        let mut bonuses = self
            .data
            .get("stat_bonuses")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        bonuses.remove(slot);
        self.set("stat_bonuses", Value::Object(bonuses));
        previous.filter(|value| !value.is_null())
    }

    pub fn equipment_total_bonus(&self, stat: &str) -> f64 {
        if self.component_type != "Equipment" {
            return 0.0;
        }
        self.data
            .get("stat_bonuses")
            .and_then(Value::as_object)
            .map(|slots| {
                slots
                    .values()
                    .filter_map(Value::as_object)
                    .map(|bonuses| bonuses.get(stat).and_then(Value::as_f64).unwrap_or(0.0))
                    .sum()
            })
            .unwrap_or(0.0)
    }

    pub fn ability_is_ready(&self, now: f64) -> bool {
        self.component_type == "Ability"
            && self.get_bool("unlocked", true)
            && self.get_i64("current_charges", 1) > 0
            && now - self.get_f64("last_cast_time", -9999.0) >= self.get_f64("cooldown", 1.0)
    }

    pub fn ability_trigger(&mut self, now: f64) -> bool {
        if !self.ability_is_ready(now) {
            return false;
        }
        let charges = (self.get_i64("current_charges", 1) - 1).max(0);
        self.set("current_charges", json!(charges));
        self.set_f64("last_cast_time", now);
        true
    }

    pub fn ability_recharge(&mut self, amount: i64) {
        if self.component_type != "Ability" {
            return;
        }
        let charges = self.get_i64("charges", 1);
        let current = self.get_i64("current_charges", 1);
        self.set("current_charges", json!((current + amount).min(charges)));
    }

    pub fn cooldown_start(&mut self, name: &str, duration: f64) {
        if self.component_type != "Cooldown" {
            return;
        }
        let mut timers = self
            .data
            .get("timers")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        timers.insert(name.to_string(), json!(duration.max(0.0)));
        self.set("timers", Value::Object(timers));
    }

    pub fn cooldown_tick(&mut self, dt: f64) {
        if self.component_type != "Cooldown" {
            return;
        }
        let timers = self
            .data
            .get("timers")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut next = Map::new();
        for (key, value) in timers {
            let remaining = (value.as_f64().unwrap_or(0.0) - dt).max(0.0);
            if remaining > 0.0 {
                next.insert(key, json!(remaining));
            }
        }
        self.set("timers", Value::Object(next));
    }

    pub fn cooldown_ready(&self, name: &str) -> bool {
        self.component_type == "Cooldown"
            && !self
                .data
                .get("timers")
                .and_then(Value::as_object)
                .map(|timers| timers.contains_key(name))
                .unwrap_or(false)
    }

    pub fn blackboard_get(&self, key: &str, default: Value) -> Value {
        if self.component_type != "Blackboard" {
            return default;
        }
        self.data
            .get("values")
            .and_then(Value::as_object)
            .and_then(|values| values.get(key))
            .cloned()
            .unwrap_or(default)
    }

    pub fn blackboard_set(&mut self, key: &str, value: Value) {
        if self.component_type != "Blackboard" {
            return;
        }
        let mut values = self
            .data
            .get("values")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        values.insert(key.to_string(), value);
        self.set("values", Value::Object(values));
    }

    pub fn blackboard_increment(&mut self, key: &str, amount: f64) -> f64 {
        let current = self.blackboard_get(key, json!(0.0)).as_f64().unwrap_or(0.0) + amount;
        self.blackboard_set(key, json!(current));
        current
    }

    pub fn state_machine_set_state(&mut self, state: &str) {
        if self.component_type != "StateMachine" {
            return;
        }
        if self.get_string("current_state", "Idle") != state {
            self.set("current_state", json!(state));
            self.set_f64("time_in_state", 0.0);
        }
    }

    pub fn quest_add(&mut self, quest_id: &str, title: &str, objectives: Value) -> bool {
        if self.component_type != "QuestLog" {
            return false;
        }
        let mut quests = self
            .data
            .get("quests")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if quests
            .iter()
            .any(|quest| quest.get("id").and_then(Value::as_str) == Some(quest_id))
        {
            return false;
        }
        quests.push(json!({
            "id": quest_id,
            "title": title,
            "state": "active",
            "objectives": objectives.as_array().cloned().unwrap_or_default(),
        }));
        self.set("quests", Value::Array(quests));
        self.set("active_quest_id", json!(quest_id));
        true
    }

    pub fn quest_set_objective_progress(
        &mut self,
        quest_id: &str,
        objective_id: &str,
        progress: Value,
    ) -> bool {
        if self.component_type != "QuestLog" {
            return false;
        }
        let mut quests = self
            .data
            .get("quests")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut changed = false;
        for quest in &mut quests {
            if quest.get("id").and_then(Value::as_str) != Some(quest_id) {
                continue;
            }
            let Some(objectives) = quest.get_mut("objectives").and_then(Value::as_array_mut) else {
                continue;
            };
            for objective in objectives {
                if objective.get("id").and_then(Value::as_str) == Some(objective_id)
                    && let Some(map) = objective.as_object_mut()
                {
                    map.insert("progress".to_string(), progress.clone());
                    changed = true;
                }
            }
        }
        if changed {
            self.set("quests", Value::Array(quests));
        }
        changed
    }

    pub fn quest_complete(&mut self, quest_id: &str) -> bool {
        if self.component_type != "QuestLog" {
            return false;
        }
        let mut quests = self
            .data
            .get("quests")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut changed = false;
        for quest in &mut quests {
            if quest.get("id").and_then(Value::as_str) == Some(quest_id)
                && let Some(map) = quest.as_object_mut()
                && map.get("state").and_then(Value::as_str) != Some("completed")
            {
                map.insert("state".to_string(), json!("completed"));
                changed = true;
            }
        }
        if changed {
            self.set("quests", Value::Array(quests));
            self.set(
                "completed_count",
                json!(self.get_i64("completed_count", 0) + 1),
            );
        }
        changed
    }

    pub fn dialogue_current_line(&self) -> String {
        if self.component_type != "Dialogue" {
            return String::new();
        }
        let lines = self
            .data
            .get("lines")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if lines.is_empty() {
            return String::new();
        }
        let index = self
            .get_i64("index", 0)
            .clamp(0, lines.len().saturating_sub(1) as i64) as usize;
        lines[index].as_str().unwrap_or("").to_string()
    }

    pub fn dialogue_advance(&mut self) -> bool {
        if self.component_type != "Dialogue" {
            return false;
        }
        let lines_len = self
            .data
            .get("lines")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let mut index = self.get_i64("index", 0) + 1;
        if index >= lines_len as i64 {
            index = lines_len.saturating_sub(1) as i64;
            self.set("is_active", json!(false));
            self.set("index", json!(index.max(0)));
            return false;
        }
        self.set("index", json!(index));
        true
    }

    pub fn dialogue_reset(&mut self) {
        if self.component_type == "Dialogue" {
            self.set("index", json!(0));
            self.set("is_active", json!(true));
        }
    }

    pub fn status_add_effect(&mut self, name: &str, duration: f64, stacks: i64, data: Value) {
        if self.component_type != "StatusEffects" {
            return;
        }
        let mut effects = self
            .data
            .get("effects")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        effects.push(json!({
            "name": name,
            "duration": duration,
            "elapsed": 0.0,
            "stacks": stacks.max(1),
            "data": data,
        }));
        self.set("effects", Value::Array(effects));
    }

    pub fn economy_add(&mut self, resource_type: &str, amount: f64) -> f64 {
        if self.component_type != "EconomyWallet" {
            return 0.0;
        }
        let mut resources = self
            .data
            .get("resources")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let current = resources
            .get(resource_type)
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let next = (current + amount).min(self.get_f64("capacity", 999999.0));
        resources.insert(resource_type.to_string(), json!(next));
        self.set("resources", Value::Object(resources));
        next
    }

    pub fn economy_spend(&mut self, resource_type: &str, amount: f64) -> bool {
        if self.component_type != "EconomyWallet" {
            return false;
        }
        let mut resources = self
            .data
            .get("resources")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let current = resources
            .get(resource_type)
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        if !self.get_bool("allow_negative", false) && current < amount {
            return false;
        }
        resources.insert(resource_type.to_string(), json!(current - amount));
        self.set("resources", Value::Object(resources));
        true
    }

    pub fn nav_set_destination(&mut self, x: f64, y: f64) {
        if self.component_type != "NavAgent" {
            return;
        }
        self.set_f64("destination_x", x);
        self.set_f64("destination_y", y);
        self.set("has_destination", json!(true));
        self.set_f64("repath_timer", 9999.0);
    }

    pub fn nav_clear_destination(&mut self) {
        if self.component_type != "NavAgent" {
            return;
        }
        self.set("has_destination", json!(false));
        self.set("last_path_length", json!(0));
    }

    pub fn camera_shake(&mut self, trauma: f64) {
        if self.component_type != "CameraShake" {
            return;
        }
        let current = self.get_f64("trauma", 0.0);
        self.set_f64("trauma", current.max(trauma));
        self.set_f64("elapsed", 0.0);
        self.set("active", json!(true));
    }

    pub fn tween_sample(&self) -> f64 {
        if self.component_type != "Tween" {
            return 0.0;
        }
        let duration = self.get_f64("duration", 1.0);
        if duration <= 0.0 {
            return self.get_f64("to_value", 1.0);
        }
        let mut t = (self.get_f64("elapsed", 0.0) / duration).clamp(0.0, 1.0);
        match self.get_string("easing", "linear").as_str() {
            "smooth" => t = t * t * (3.0 - 2.0 * t),
            "ease_in" => t *= t,
            "ease_out" => t = 1.0 - (1.0 - t) * (1.0 - t),
            _ => {}
        }
        let from = self.get_f64("from_value", 0.0);
        let to = self.get_f64("to_value", 1.0);
        from + (to - from) * t
    }

    pub fn timer_tick(&mut self, dt: f64) -> bool {
        if self.component_type != "Timer"
            || !self.get_bool("running", true)
            || self.get_bool("completed", false)
        {
            return false;
        }
        let elapsed = self.get_f64("elapsed", 0.0) + dt.max(0.0);
        self.set_f64("elapsed", elapsed);
        if elapsed < self.get_f64("duration", 1.0) {
            return false;
        }
        if self.get_bool("loop", false) {
            self.set_f64("elapsed", 0.0);
        } else {
            self.set("completed", json!(true));
            self.set("running", json!(false));
        }
        true
    }

    pub fn damage_can_hit(&self, entity_id: u64, now: f64) -> bool {
        if self.component_type != "DamageDealer" {
            return false;
        }
        let hits = self
            .data
            .get("last_hits")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let key = entity_id.to_string();
        if self.get_bool("hit_once", false) && hits.contains_key(&key) {
            return false;
        }
        let last = hits.get(&key).and_then(Value::as_f64).unwrap_or(-9999.0);
        now - last >= self.get_f64("cooldown", 0.5)
    }

    pub fn damage_mark_hit(&mut self, entity_id: u64, now: f64) {
        if self.component_type != "DamageDealer" {
            return;
        }
        let mut hits = self
            .data
            .get("last_hits")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        hits.insert(entity_id.to_string(), json!(now));
        self.set("last_hits", Value::Object(hits));
    }
}

/// Runtime bookkeeping uses underscore-prefixed keys. Those values must never
/// leak into scenes, prefabs or undo snapshots, including when nested in a
/// component-owned object such as VisualScript variables.
fn persistent_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(key, _)| !key.starts_with('_'))
                .map(|(key, value)| (key.clone(), persistent_value(value)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(persistent_value).collect()),
        _ => value.clone(),
    }
}

pub fn component_from_data(data: &Value) -> Option<Component> {
    let component_type = data.get("component_type")?.as_str()?;
    let mut component = default_component(component_type)?;
    component.merge_data(data);
    Some(component)
}

pub fn default_component(component_type: &str) -> Option<Component> {
    let defaults = match component_type {
        "Transform" => json!({
            "x": 0.0,
            "y": 0.0,
            "rotation": 0.0,
            "scale_x": 1.0,
            "scale_y": 1.0,
        }),
        "Actor2D" => json!({
            "class": "Actor2D",
            "guid": null,
            "replicate": false,
            "tick_enabled": true,
            "tick_group": "default",
            "folder": "",
            "labels": [],
        }),
        "GameMode2D" => json!({
            "default_pawn": "BP_PlayerPawn2D",
            "player_controller": "BP_PlayerController2D",
            "hud_canvas": "assets/ui/hud.ui2d.json",
            "start_scene": "saves/scenes/main.scene",
            "spawn_policy": "spawn_at_player_start",
            "rules": {},
        }),
        "GameState2D" => json!({
            "elapsed_time": 0.0,
            "paused": false,
            "score": {},
            "phase": "playing",
        }),
        "PlayerState2D" => json!({
            "player_id": 0,
            "display_name": "Player",
            "score": 0,
            "lives": 3,
        }),
        "Pawn2D" => json!({
            "auto_possess": true,
            "controller_id": null,
            "movement_mode": "topdown",
            "input_enabled": true,
            "camera_follow": true,
        }),
        "Controller2D" => json!({
            "possessed_pawn": null,
            "enabled": true,
            "input_context": null,
            "control_scheme": "default",
        }),
        "PlayerController2D" => json!({
            "possessed_pawn": null,
            "input_context": "settings/input_map.json",
            "cursor_visible": true,
            "click_to_move": false,
            "ui_focus": null,
        }),
        "AIController2D" => json!({
            "possessed_pawn": null,
            "behavior_tree": "assets/ai/basic_enemy.bt2d.json",
            "blackboard": {},
            "think_interval": 0.25,
            "state": "idle",
        }),
        "AssetIdentity2D" => json!({
            "guid": null,
            "asset_type": "DataAsset2D",
            "labels": [],
            "preview": null,
            "dependencies": [],
            "valid": true,
        }),
        "SpriteRenderer" => json!({
            "sprite_name": null,
            "sprite_path": null,
            "sprite_guid": null,
            "source_asset": null,
            "material": "Default",
            "material_path": null,
            "visible": true,
            "sorting_order": 0,
            "flip_x": false,
            "flip_y": false,
            "pivot_x": 0.5,
            "pivot_y": 0.5,
            "tint": [255, 255, 255],
        }),
        "RTSMovement" => json!({
            "speed": 3.5,
            "separation": true,
            "allow_pathfinding": true,
            "formation_role": "unit",
            "acceleration": 1.0,
            "turn_speed": 1.0,
        }),
        "Selectable" => json!({
            "selectable": true,
            "selection_radius": 0.5,
        }),
        "MovementComponent" => json!({
            "speed_x": 0.01,
            "speed_y": 0.0,
        }),
        "AudioSource" => json!({
            "audio_name": null,
            "volume": 1.0,
            "pitch": 1.0,
            "bus": "SFX",
            "spatial_blend": 0.0,
            "min_distance": 4.0,
            "max_distance": 18.0,
            "play_on_start": false,
            "loop": false,
            "priority": 128,
        }),
        "AudioSource2D" => json!({
            "audio_name": null,
            "volume": 1.0,
            "pitch": 1.0,
            "bus": "SFX",
            "spatial_blend": 0.0,
            "min_distance": 4.0,
            "max_distance": 18.0,
            "play_on_start": false,
            "loop": false,
            "priority": 128,
        }),
        "Rigidbody2D" => json!({
            "body_type": "dynamic",
            "velocity_x": 0.0,
            "velocity_y": 0.0,
            "angular_velocity": 0.0,
            "mass": 1.0,
            "gravity_scale": 1.0,
            "gravity_x": null,
            "gravity_y": null,
            "drag": 0.05,
            "angular_drag": 0.05,
            "bounciness": 0.0,
            "friction": 0.25,
            "use_gravity": true,
            "freeze_x": false,
            "freeze_y": false,
            "freeze_rotation": false,
            "continuous_collision": false,
            "sleeping": false,
        }),
        "StaticBody2D" => json!({
            "body_type": "static",
            "friction": 0.4,
            "bounciness": 0.0,
            "one_way": false,
            "one_way_normal_x": 0.0,
            "one_way_normal_y": -1.0,
        }),
        "KinematicBody2D" => json!({
            "body_type": "kinematic",
            "velocity_x": 0.0,
            "velocity_y": 0.0,
            "move_and_slide": true,
        }),
        "CharacterBody2D" => json!({
            "mode": "platformer",
            "velocity_x": 0.0,
            "velocity_y": 0.0,
            "max_speed": 7.0,
            "acceleration": 40.0,
            "deceleration": 50.0,
            "floor_snap": 0.08,
            "max_slope_degrees": 45.0,
            "grounded": false,
            "collision_layer": "Pawn",
            "collision_mask": ["WorldStatic", "OneWayPlatform", "Trigger"],
        }),
        "Animator" => json!({
            "controller": "Default",
            "current_state": "Idle",
            "speed": 1.0,
            "play_on_start": true,
            "loop": true,
            "parameters": {},
            "preview": true,
            "apply_sprite": true,
            "apply_tint": true,
            "normalized_time": 0.0,
        }),
        "Animator2D" => json!({
            "controller": "Default",
            "animation_blueprint": "assets/animations/ABP_Player2D.anim2d.json",
            "current_state": "Idle",
            "parameters": {},
            "triggers": [],
            "preview": true,
        }),
        "AnimatedSprite" => json!({
            "frames": "assets/animations/player.spriteframes",
            "animation": "idle",
            "playing": true,
            "speed": 1.0,
            "loop": true,
            "flip_x": false,
            "flip_y": false,
            "frame": 0,
            "frame_events": [],
        }),
        "AnimationPlayer" => json!({
            "current": "Idle",
            "playing": false,
            "speed": 1.0,
            "loop": false,
            "parameters": {},
            "states": ["Idle"],
            "transitions": [],
            "events": [],
            "property_tracks": [],
        }),
        "Camera2D" => json!({
            "active": false,
            "zoom": 1.0,
            "clear_color": [18, 20, 24, 255],
            "follow_target": null,
            "smooth": true,
            "smoothness": 8.0,
            "limits": {"enabled": false, "min_x": 0.0, "min_y": 0.0, "max_x": 0.0, "max_y": 0.0},
            "screen_shake": {"active": false, "duration": 0.0, "elapsed": 0.0, "amplitude": 0.0},
            "pixel_perfect": false,
            "pixels_per_unit": 16.0,
            "viewport_width": 1280.0,
            "viewport_height": 720.0,
        }),
        "Transform3D" => json!({
            "x": 0.0,
            "y": 0.0,
            "z": 0.0,
            "rotation_x": 0.0,
            "rotation_y": 0.0,
            "rotation_z": 0.0,
            "scale_x": 1.0,
            "scale_y": 1.0,
            "scale_z": 1.0,
            "inherit_2d_transform": false,
        }),
        "MeshRenderer3D" => json!({
            "mesh": "builtin:cube",
            "material": "Default3D",
            "visible": true,
            "cast_shadows": false,
            "receive_shadows": true,
            "layer": "World3D",
            "lod_group": "default",
        }),
        "Camera3D" => json!({
            "active": false,
            "projection": "perspective",
            "x": 0.0,
            "y": 4.0,
            "z": 8.0,
            "target_x": 0.0,
            "target_y": 0.0,
            "target_z": 0.0,
            "up_x": 0.0,
            "up_y": 1.0,
            "up_z": 0.0,
            "fov_y_degrees": 60.0,
            "near": 0.05,
            "far": 500.0,
            "renders_2d_overlay": true,
        }),
        "Light3D" => json!({
            "light_type": "directional",
            "color": [255, 245, 225],
            "intensity": 1.0,
            "range": 64.0,
            "direction_x": -0.4,
            "direction_y": -1.0,
            "direction_z": -0.3,
            "casts_shadows": false,
        }),
        "Material3D" => json!({
            "material": "Default3D",
            "shader": "standard_lit_3d",
            "albedo": [255, 255, 255, 255],
            "albedo_texture": null,
            "normal_map": null,
            "metallic": 0.0,
            "roughness": 0.75,
            "cull_mode": "back",
            "depth_write": true,
        }),
        "Billboard3D" => json!({
            "sprite": null,
            "face_camera": true,
            "lock_y_axis": false,
            "width": 1.0,
            "height": 1.0,
            "sorting_bias": 0,
            "use_2d_animation": true,
        }),
        "HybridScene3D" => json!({
            "enabled": false,
            "render_2d_overlay": true,
            "depth_buffer": true,
            "physics_mode": "2d_gameplay",
            "world_scale": 1.0,
            "notes": "Preview 3D inicial; gameplay 2D sigue siendo la ruta estable.",
        }),
        "WorldPartition2D" => json!({
            "cell_size": 64.0,
            "load_radius_cells": 2,
            "keepalive_radius_cells": 3,
            "max_loaded_chunks": 49,
            "chunk_folder": "saves/scenes/chunks",
            "streaming_enabled": true,
        }),
        "StreamingChunk2D" => json!({
            "cell_x": 0,
            "cell_y": 0,
            "scene_path": "saves/scenes/chunks/chunk_0_0.scene",
            "priority": 0,
            "loaded": false,
            "entity_count": 0,
            "last_touched_frame": 0,
        }),
        "RuntimeBudget2D" => json!({
            "target_fps": 60,
            "max_entities": 20000,
            "max_visible_sprites": 8000,
            "max_particles": 25000,
            "max_draw_calls": 500,
            "max_loaded_chunks": 49,
            "max_script_ms": 4.0,
            "max_physics_ms": 4.0,
            "max_ui_ms": 2.0,
            "max_memory_mb": 1024.0,
        }),
        "ObjectPool2D" => json!({
            "buckets": [
                {"prefab": "assets/prefabs/projectile.prefab", "warm": 128, "active": 0, "inactive": 128, "hard_limit": 1024}
            ],
            "enabled": true,
        }),
        "SpawnDirector2D" => json!({
            "max_spawn_per_tick": 8,
            "rules": [
                {"prefab": "assets/prefabs/enemy.prefab", "tag": "Enemy", "min_distance_from_camera": 12.0, "max_distance_from_camera": 24.0, "max_alive": 80, "weight": 1.0, "cooldown_frames": 30, "last_spawn_frame": 0}
            ],
            "enabled": true,
        }),
        "SaveShard2D" => json!({
            "shard_size_cells": 4,
            "global_save_path": "saves/profile/global.json",
            "dirty_cells": [],
            "autosave_dirty_shards": true,
        }),
        "VisualScript" => json!({
            "graph_name": "NewGraph",
            "run_in_editor": false,
            "variables": {},
            "nodes": [
                {"id": "start", "type": "EventStart", "next": "log"},
                {"id": "log", "type": "Log", "message": "VisualScript started", "next": null}
            ],
            "enabled_events": ["start", "update", "collision"],
        }),
        "ScriptComponent" => json!({
            "runtime": "luau",
            "path": null,
            "scripts": [],
            "public_variables": {},
            "hot_reload": true,
            "last_error": null,
        }),
        "ScriptSchedule" => json!({
            "enabled": true,
            "always_update": false,
            "update_interval": 0.0,
            "max_distance": 0.0,
            "distant_update_interval": 0.75,
            "priority": 0,
        }),
        "VisualGraphComponent" => json!({
            "runtime": "miniforge_visual_script_2d",
            "path": null,
            "public_variables": {},
            "debug_runtime": true,
            "last_error": null,
        }),
        "TilemapRenderer2D" => json!({
            "tilemap": "assets/tilemaps/demo.tilemap.json",
            "tileset": "assets/tiles/demo.tileset.json",
            "visible_layers": ["Ground"],
            "sorting_order": 0,
            "collision_layer": "WorldStatic",
            "debug_grid": false,
        }),
        "Tilemap2D" => json!({
            "width": 32,
            "height": 18,
            "tile_width": 16,
            "tile_height": 16,
            "chunk_width": 16,
            "chunk_height": 16,
            "layers": [
                {"name": "Ground", "visible": true, "collision": false, "navigation": false, "tiles": []},
                {"name": "Collision", "visible": false, "collision": true, "navigation": false, "tiles": []},
                {"name": "Navigation", "visible": false, "collision": false, "navigation": true, "tiles": []}
            ],
            "autotiles": [],
            "animated_tiles": [],
            "dirty_chunks": [],
        }),
        "TilemapChunk2D" => json!({
            "chunk_x": 0,
            "chunk_y": 0,
            "width": 16,
            "height": 16,
            "dirty": true,
            "visible": true,
        }),
        "Tileset2D" => json!({
            "texture": "assets/tiles/demo_tiles.png",
            "tile_width": 16,
            "tile_height": 16,
            "columns": 8,
            "collision_tiles": [],
        }),
        "FlipbookAnimation2D" => json!({
            "flipbook": "assets/animations/player_run.flipbook.json",
            "current_time": 0.0,
            "frames_per_second": 10.0,
            "looping": true,
            "playing": true,
            "frame_events_enabled": true,
        }),
        "AnimationBlueprint2D" => json!({
            "asset": "assets/animations/ABP_Player2D.anim2d.json",
            "current_state": "Idle",
            "parameters": {},
            "frame_events": [],
            "apply_to_animator": true,
        }),
        "UIElement" => json!({
            "element_type": "Label",
            "text": "Label",
            "anchor": "top_left",
            "x": 24.0,
            "y": 24.0,
            "width": 160.0,
            "height": 36.0,
            "color": [245, 247, 252],
            "text_color": [35, 36, 42],
            "image_name": null,
            "opacity": 1.0,
            "interactable": false,
            "on_click_graph": null,
            "sorting_order": 0,
            "padding": 8,
            "border_radius": 7,
            "border_color": [180, 185, 198],
            "text_align": "center",
            "font_size": 0,
            "progress": 1.0,
            "max_progress": 1.0,
        }),
        "WidgetCanvas2D" => json!({
            "canvas": "assets/ui/hud.ui2d.json",
            "visible": true,
            "input_enabled": true,
            "scale_mode": "scale_with_screen",
            "callbacks_enabled": true,
        }),
        "Sequencer2D" => json!({
            "sequence": "assets/sequences/intro.seq2d.json",
            "playing": false,
            "time": 0.0,
            "duration": 0.0,
            "loop": false,
            "auto_play": false,
        }),
        "Collider2D" => json!({
            "shape": "rect",
            "width": 1.0,
            "height": 1.0,
            "radius": 0.5,
            "points": [[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]],
            "is_trigger": false,
            "offset_x": 0.0,
            "offset_y": 0.0,
            "collision_layer": "Default",
            "collision_mask": ["*"],
            "material": {
                "friction": 0.25,
                "bounciness": 0.0
            },
        }),
        "Area2D" => json!({
            "monitoring": true,
            "monitorable": true,
            "shape": "rect",
            "width": 1.0,
            "height": 1.0,
            "radius": 0.5,
            "collision_layer": "Trigger",
            "collision_mask": ["Pawn"],
            "entered": [],
            "exited": [],
        }),
        "OneWayPlatform2D" => json!({
            "enabled": true,
            "normal_x": 0.0,
            "normal_y": -1.0,
            "pass_through_from_below": true,
            "surface_margin": 0.08,
        }),
        "Trigger2D" => json!({
            "shape": "rect",
            "width": 1.0,
            "height": 1.0,
            "radius": 0.5,
            "layer": "Trigger",
            "overlap_mask": ["Pawn"],
            "on_enter_graph": null,
            "on_exit_graph": null,
        }),
        "Health" => json!({
            "max_health": 100.0,
            "health": 100.0,
            "armor": 0.0,
            "alive": true,
        }),
        "Team" => json!({
            "team_id": 0,
            "team_name": "Neutral",
            "color": [80, 120, 255],
        }),
        "RTSController" => json!({
            "team_id": 1,
            "camera_mode": "rts",
            "selected_ids": [],
            "control_groups": {},
            "command_mode": "smart",
        }),
        "Commandable" => json!({
            "can_move": true,
            "can_attack": true,
            "can_gather": false,
            "can_build": false,
            "can_produce": false,
            "command_tags": ["move", "stop", "hold", "patrol", "attack_move"],
        }),
        "SquadMember" => json!({
            "squad_id": null,
            "slot": 0,
            "role": "line",
            "cohesion_radius": 4.0,
            "formation_weight": 1.0,
        }),
        "RtsBrain" => json!({
            "enabled": true,
            "strategy": "balanced",
            "build_order": ["Worker", "Soldier"],
            "attack_threshold": 6,
            "retreat_health_pct": 0.25,
            "scout_interval": 20.0,
            "last_decision_time": 0.0,
        }),
        "ProductionRecipeBook" => json!({
            "recipes": [
                {"unit_type": "Worker", "display_name": "Worker", "build_time": 3.0, "cost": {"Gold": 50.0}},
                {"unit_type": "Soldier", "display_name": "Soldier", "build_time": 5.0, "cost": {"Gold": 85.0, "Wood": 25.0}}
            ],
            "auto_queue": false,
            "preferred_recipe": "Worker",
        }),
        "Vision" => json!({
            "radius": 7.0,
            "reveals_fog": true,
            "detector": false,
            "team_shared": true,
        }),
        "FogOfWar" => json!({
            "team_id": 1,
            "map_width": 60,
            "map_height": 40,
            "tile_size": 1.0,
            "visible_tiles": [],
            "explored_tiles": [],
        }),
        "ThreatSource" => json!({
            "strength": 8.0,
            "radius": 5.0,
            "falloff": 1.0,
            "avoidance_weight": 24.0,
            "affects_teams": [],
            "enabled": true,
        }),
        "InfluenceSource" => json!({
            "team_id": 1,
            "strength": 10.0,
            "falloff": 2.0,
            "label": "Control",
            "enabled": true,
        }),
        "ResourceNode" => json!({
            "resource_type": "Gold",
            "amount": 500.0,
            "max_amount": 500.0,
            "gather_rate": 10.0,
            "harvest_radius": 1.25,
        }),
        "Worker" => json!({
            "carry_capacity": 50.0,
            "carrying_type": null,
            "carrying_amount": 0.0,
            "gather_target_id": null,
            "gather_range": 1.35,
            "gather_efficiency": 1.0,
            "auto_deposit": true,
        }),
        "ProductionQueue" => json!({
            "queue": [],
            "max_queue": 7,
            "rally_x": 0.0,
            "rally_y": 0.0,
            "auto_start": true,
            "production_speed": 1.0,
            "blocked_reason": null,
        }),
        "Buildable" => json!({
            "display_name": "Building",
            "footprint_w": 2,
            "footprint_h": 2,
            "build_time": 8.0,
            "cost": {"Gold": 150.0, "Wood": 50.0},
            "produces": [],
            "requires_power": false,
        }),
        "ConstructionSite" => json!({
            "target_name": "Building",
            "target_tag": "Building",
            "progress": 0.0,
            "build_time": 8.0,
            "build_rate": 1.0,
            "builder_ids": [],
            "completed": false,
            "finished_components": ["Health", "ProductionQueue", "Vision"],
        }),
        "Stats" => json!({
            "level": 1,
            "experience": 0.0,
            "experience_to_next": 100.0,
            "strength": 5.0,
            "agility": 5.0,
            "intelligence": 5.0,
            "vitality": 5.0,
            "attack": 10.0,
            "defense": 0.0,
            "magic": 0.0,
            "resistance": 0.0,
            "move_speed_bonus": 0.0,
            "max_health_bonus": 0.0,
            "critical_chance": 0.05,
            "critical_multiplier": 1.5,
            "regen_per_second": 0.0,
        }),
        "Inventory" => json!({
            "capacity": 24,
            "items": [],
            "currency": {},
            "stack_limit": 99,
            "locked": false,
        }),
        "Equipment" => json!({
            "slots": {"weapon": null, "armor": null, "trinket": null, "tool": null},
            "stat_bonuses": {},
            "locked_slots": [],
        }),
        "Ability" => json!({
            "ability_id": "ability",
            "display_name": "Ability",
            "cooldown": 1.0,
            "mana_cost": 0.0,
            "range": 4.0,
            "power": 10.0,
            "target_mode": "entity",
            "charges": 1,
            "current_charges": 1,
            "recharge_time": 0.0,
            "last_cast_time": -9999.0,
            "unlocked": true,
            "tags": [],
        }),
        "AIController" => json!({
            "behavior": "idle",
            "target_id": null,
            "home_x": 0.0,
            "home_y": 0.0,
            "think_interval": 0.25,
            "think_timer": 0.0,
            "detection_radius": 6.0,
            "attack_radius": 1.25,
            "leash_radius": 12.0,
            "patrol_radius": 4.0,
            "wander_radius": 5.0,
            "target_tags": ["Enemy", "Player"],
            "state": "idle",
        }),
        "NavAgent" => json!({
            "has_destination": false,
            "destination_x": 0.0,
            "destination_y": 0.0,
            "speed": 3.5,
            "stopping_distance": 0.15,
            "repath_interval": 0.25,
            "repath_timer": 0.0,
            "auto_repath": true,
            "avoid_obstacles": true,
            "path_smoothing": true,
            "last_path_length": 0,
        }),
        "Interaction" => json!({
            "prompt": "Interact",
            "radius": 1.25,
            "action_name": "interact",
            "action_graph": null,
            "requires_tag": "Player",
            "single_use": false,
            "used": false,
            "active": false,
        }),
        "Lifetime" => json!({
            "duration": 5.0,
            "elapsed": 0.0,
            "destroy_on_expire": true,
            "fade_out": false,
        }),
        "Spawner" => json!({
            "prefab_name": "",
            "spawn_interval": 5.0,
            "spawn_radius": 2.0,
            "max_alive": 3,
            "spawn_on_start": false,
            "enabled_in_editor": false,
            "spawned_ids": [],
            "elapsed": 0.0,
            "started": false,
        }),
        "DamageDealer" => json!({
            "damage": 10.0,
            "range": 1.25,
            "damage_type": "physical",
            "cooldown": 0.5,
            "knockback": 0.0,
            "target_tags": ["Enemy"],
            "hit_once": false,
            "last_hits": {},
        }),
        "CameraFollow" => json!({
            "target_id": null,
            "smoothness": 8.0,
            "zoom_smoothness": 10.0,
            "offset_x": 0.0,
            "offset_y": 0.0,
            "zoom": 1.0,
            "dead_zone": 0.0,
            "follow_x": true,
            "follow_y": true,
            "viewport_width": 960.0,
            "viewport_height": 540.0,
        }),
        "Saveable" => json!({
            "save_key": "",
            "include_components": true,
            "persistent": true,
            "version": 1,
            "autosave": true,
        }),
        "Blackboard" => json!({"values": {}}),
        "InputActions2D" => json!({
            "actions": {
                "move_left": ["A", "Left"],
                "move_right": ["D", "Right"],
                "move_up": ["W", "Up"],
                "move_down": ["S", "Down"],
                "jump": ["Space"],
                "fire": ["MouseLeft", "Ctrl"]
            }
        }),
        "EventBus2D" => json!({
            "subscriptions": {},
            "last_events": [],
        }),
        "BehaviorTree2D" => json!({
            "tree": "assets/ai/basic_enemy.bt2d.json",
            "running": true,
            "root": "root",
            "active_node": null,
            "last_status": "idle",
            "tasks": ["Patrol", "Chase", "Attack", "Flee", "RTSCommand"],
        }),
        "StateMachine" => json!({
            "current_state": "Idle",
            "initial_state": "Idle",
            "states": ["Idle"],
            "transitions": [],
            "time_in_state": 0.0,
            "auto_start": true,
        }),
        "QuestLog" => json!({
            "quests": [],
            "active_quest_id": null,
            "completed_count": 0,
        }),
        "Dialogue" => json!({
            "speaker": "NPC",
            "lines": ["Hello."],
            "index": 0,
            "is_active": false,
            "auto_advance": false,
            "choices": [],
            "on_complete_graph": null,
        }),
        "Cooldown" => json!({"timers": {}}),
        "StatusEffects" => json!({"effects": []}),
        "CombatTarget" => json!({
            "target_id": null,
            "aggro_radius": 6.0,
            "attack_radius": 1.25,
            "lose_radius": 10.0,
            "target_tags": ["Enemy"],
            "require_line_of_sight": false,
        }),
        "LootTable" => json!({
            "entries": [{"id": "coin", "weight": 1.0, "min": 1, "max": 3}],
            "rolls": 1,
            "drop_radius": 0.5,
            "guaranteed_currency": {},
        }),
        "CameraShake" => json!({
            "amplitude": 6.0,
            "duration": 0.25,
            "frequency": 24.0,
            "elapsed": 0.0,
            "trauma": 0.0,
            "active": false,
        }),
        "Light2D" => json!({
            "light_type": "point",
            "color": [255, 240, 200],
            "radius": 5.0,
            "intensity": 1.0,
            "falloff": 1.0,
            "angle": 360.0,
            "direction": 0.0,
            "flicker": false,
            "flicker_speed": 6.0,
            "casts_shadows": true,
            "shadow_softness": 0.35,
            "shadow_bias": 0.01,
        }),
        "ShadowCaster2D" => json!({
            "shape": "sprite_alpha",
            "points": [],
            "opacity": 0.8,
            "two_sided": true,
            "self_shadow": false,
        }),
        "NormalMap2D" => json!({
            "normal_texture": null,
            "strength": 1.0,
            "flip_y": false,
            "generate_from_height": false,
            "height_scale": 1.0,
        }),
        "Water2D" => json!({
            "shader": "water_2d",
            "wave_strength": 0.15,
            "wave_speed": 1.2,
            "refraction": 0.08,
            "foam_amount": 0.4,
            "tint": [55, 145, 210, 190],
            "normal_texture": null,
        }),
        "Distortion2D" => json!({
            "mode": "heat",
            "strength": 0.06,
            "speed": 1.0,
            "frequency": 2.5,
            "mask_texture": null,
        }),
        "Fire2D" => json!({
            "particle_template": "Fire2D",
            "gpu_preferred": true,
            "heat_distortion": true,
            "emission": [255, 95, 20],
            "intensity": 2.0,
        }),
        "Fog2D" => json!({
            "mode": "height",
            "color": [95, 115, 140, 150],
            "density": 0.18,
            "height_falloff": 0.35,
            "noise_strength": 0.3,
            "noise_speed": 0.08,
        }),
        "Outline2D" => json!({
            "color": [20, 22, 28, 255],
            "width": 1.0,
            "mode": "alpha_edge",
            "inside": false,
        }),
        "Bloom2D" => json!({
            "threshold": 0.8,
            "intensity": 0.7,
            "radius": 4.0,
            "quality": "balanced",
        }),
        "GpuParticles2D" => json!({
            "template": "MagicAura2D",
            "max_particles": 100000,
            "simulation": "gpu_preferred",
            "fallback": "cpu",
            "local_space": false,
        }),
        "DamageEffect2D" => json!({
            "flash_color": [255, 65, 65, 255],
            "flash_duration": 0.12,
            "shake_amplitude": 5.0,
            "chromatic_aberration": 0.04,
            "vignette": 0.3,
        }),
        "PixelArtShader2D" => json!({
            "palette_mode": "indexed",
            "palette_size": 16,
            "dither": "bayer4x4",
            "pixel_scale": 1.0,
            "nearest_filter": true,
            "snap_uv": true,
        }),
        "Material2D" => json!({
            "material": "Default",
            "material_path": null,
            "shader": "sprite_default",
            "tint": [255, 255, 255, 255],
            "texture": null,
            "base_color_texture": null,
            "normal_texture": null,
            "roughness_texture": null,
            "metallic_texture": null,
            "emissive_texture": null,
            "texture_parameters": {},
            "lighting": false,
            "fog": false,
            "roughness": 0.5,
            "emission": [0, 0, 0],
        }),
        "ParticleEmitter" => crate::systems::particle_system::default_particle_emitter(),
        "ParallaxLayer" => json!({
            "factor_x": 0.5,
            "factor_y": 0.5,
            "offset_x": 0.0,
            "offset_y": 0.0,
            "repeat_x": true,
            "repeat_y": false,
            "sorting_order": -10,
        }),
        "TilemapCollider" => json!({
            "solid_tiles": [1, 3, 4],
            "one_way_tiles": [],
            "friction": 0.4,
            "bounciness": 0.0,
            "enabled_layers": ["Ground"],
        }),
        "ObjectiveMarker" => json!({
            "label": "Objective",
            "color": [255, 210, 90],
            "visible": true,
            "max_distance": 9999.0,
            "pulse": true,
            "target_id": null,
        }),
        "Checkpoint" => json!({
            "checkpoint_id": "checkpoint",
            "respawn_x": 0.0,
            "respawn_y": 0.0,
            "respawn_health": 100.0,
            "activation_radius": 1.2,
            "active": false,
            "single_use": false,
            "activated_by_tag": "Player",
        }),
        "DontDestroyOnLoad" => json!({
            "preserve": true,
            "group": "global",
        }),
        "CharacterController2D" => json!({
            "mode": "auto",
            "walk_speed": 5.0,
            "run_speed": 7.0,
            "jump_force": 9.0,
            "grounded": false,
            "coyote_time": 0.12,
            "coyote_timer": 0.0,
            "jump_buffer_time": 0.12,
            "jump_buffer_timer": 0.0,
            "jump_cut_multiplier": 0.55,
            "air_control": 0.6,
            "max_jumps": 1,
            "jumps_used": 0,
            "input_x": 0.0,
            "input_y": 0.0,
            "jump_pressed": false,
            "jump_held": false,
            "run_pressed": false,
            "dash_pressed": false,
            "dash_speed": 12.0,
            "dash_duration": 0.12,
            "dash_timer": 0.0,
            "dash_cooldown": 0.45,
            "dash_cooldown_timer": 0.0,
            "dashing": false,
            "moving": false,
            "facing_x": 1.0,
            "facing_y": 0.0,
            "fall_death_y": 9999.0,
            "input_enabled": true,
        }),
        "EconomyWallet" => json!({
            "resources": {"Gold": 0},
            "capacity": 999999.0,
            "allow_negative": false,
        }),
        "Province2D" => json!({
            "province_id": "province",
            "display_name": "Province",
            "owner_tag": "Neutral",
            "controller_tag": "Neutral",
            "terrain": "plains",
            "population": 100000.0,
            "literacy": 0.15,
            "infrastructure": 1.0,
            "resource": "grain",
            "factory_slots": 1,
            "supply_limit": 10.0,
            "neighbors": [],
            "selected": false,
        }),
        "Nation2D" => json!({
            "nation_tag": "NAT",
            "display_name": "Nation",
            "capital_province": null,
            "government": "constitutional_monarchy",
            "ruling_party": "liberal",
            "prestige": 0.0,
            "infamy": 0.0,
            "tax_rate": 0.25,
            "tariff_rate": 0.05,
            "treasury": 1000.0,
            "accepted_cultures": [],
            "primary_culture": "core",
            "national_focus": "industrialize",
        }),
        "PopulationPops2D" => json!({
            "pops": [
                {"type": "farmers", "size": 65000.0, "militancy": 0.05, "consciousness": 0.10, "wealth": 0.35},
                {"type": "craftsmen", "size": 12000.0, "militancy": 0.08, "consciousness": 0.25, "wealth": 0.45}
            ],
            "migration_pull": 0.0,
            "assimilation_rate": 0.01,
            "growth_rate": 0.012,
        }),
        "Market2D" => json!({
            "market_id": "local_market",
            "goods": {
                "grain": {"stockpile": 100.0, "price": 1.0, "demand": 80.0, "supply": 100.0},
                "iron": {"stockpile": 30.0, "price": 2.0, "demand": 45.0, "supply": 30.0}
            },
            "tariffs_enabled": true,
            "auto_price_update": true,
        }),
        "Factory2D" => json!({
            "factory_id": "factory",
            "good": "steel",
            "level": 1,
            "workers": 0.0,
            "throughput": 1.0,
            "inputs": {"coal": 1.0, "iron": 1.0},
            "output": 1.0,
            "subsidized": false,
            "profit": 0.0,
        }),
        "Diplomacy2D" => json!({
            "relations": {},
            "alliances": [],
            "rivals": [],
            "truce_until": {},
            "influence": {},
            "sphere_leader": null,
        }),
        "ResearchTree2D" => json!({
            "current_research": null,
            "progress": 0.0,
            "points_per_month": 1.0,
            "unlocked": [],
            "available": ["steam_power", "organized_factories", "professional_army"],
        }),
        "ArmyStack2D" => json!({
            "army_id": "army",
            "owner_tag": "NAT",
            "province_id": null,
            "regiments": [
                {"type": "infantry", "strength": 3000.0, "organization": 1.0, "morale": 1.0}
            ],
            "general": null,
            "movement_order": null,
            "supply": 1.0,
            "dig_in": 0.0,
        }),
        "WarGoal2D" => json!({
            "war_id": null,
            "attacker_tag": null,
            "defender_tag": null,
            "goal_type": "conquest",
            "target_province": null,
            "war_score": 0.0,
            "active": false,
        }),
        "TradeRoute2D" => json!({
            "route_id": "trade_route",
            "from_market": null,
            "to_market": null,
            "good": "grain",
            "volume": 0.0,
            "capacity": 100.0,
            "profit": 0.0,
            "risk": 0.0,
        }),
        "Timer" => json!({
            "name": "Timer",
            "duration": 1.0,
            "elapsed": 0.0,
            "loop": false,
            "running": true,
            "completed": false,
        }),
        "Tween" => json!({
            "property_path": "x",
            "from_value": 0.0,
            "to_value": 1.0,
            "duration": 1.0,
            "elapsed": 0.0,
            "easing": "linear",
            "loop": false,
            "ping_pong": false,
            "active": false,
        }),
        _ => return None,
    };

    Some(Component::with_defaults(component_type, defaults))
}

pub fn advanced_component_types() -> &'static [&'static str] {
    &[
        "Actor2D",
        "GameMode2D",
        "GameState2D",
        "PlayerState2D",
        "Pawn2D",
        "Controller2D",
        "PlayerController2D",
        "AIController2D",
        "AssetIdentity2D",
        "TilemapRenderer2D",
        "Tilemap2D",
        "TilemapChunk2D",
        "Tileset2D",
        "FlipbookAnimation2D",
        "AnimatedSprite",
        "AnimationPlayer",
        "AnimationBlueprint2D",
        "Animator2D",
        "ScriptComponent",
        "ScriptSchedule",
        "VisualGraphComponent",
        "AudioSource2D",
        "Camera2D",
        "Transform3D",
        "MeshRenderer3D",
        "Camera3D",
        "Light3D",
        "Material3D",
        "Billboard3D",
        "HybridScene3D",
        "WorldPartition2D",
        "StreamingChunk2D",
        "RuntimeBudget2D",
        "ObjectPool2D",
        "SpawnDirector2D",
        "SaveShard2D",
        "WidgetCanvas2D",
        "Sequencer2D",
        "Area2D",
        "OneWayPlatform2D",
        "Trigger2D",
        "StaticBody2D",
        "KinematicBody2D",
        "CharacterBody2D",
        "BehaviorTree2D",
        "Stats",
        "Inventory",
        "Equipment",
        "Ability",
        "RTSController",
        "Commandable",
        "SquadMember",
        "RtsBrain",
        "ProductionRecipeBook",
        "Vision",
        "FogOfWar",
        "ThreatSource",
        "InfluenceSource",
        "ProductionQueue",
        "Buildable",
        "ConstructionSite",
        "AIController",
        "NavAgent",
        "Interaction",
        "Lifetime",
        "Spawner",
        "DamageDealer",
        "CameraFollow",
        "Saveable",
        "Blackboard",
        "InputActions2D",
        "EventBus2D",
        "StateMachine",
        "QuestLog",
        "Dialogue",
        "Province2D",
        "Nation2D",
        "PopulationPops2D",
        "Market2D",
        "Factory2D",
        "Diplomacy2D",
        "ResearchTree2D",
        "ArmyStack2D",
        "WarGoal2D",
        "TradeRoute2D",
        "Cooldown",
        "StatusEffects",
        "CombatTarget",
        "LootTable",
        "CameraShake",
        "Light2D",
        "ShadowCaster2D",
        "NormalMap2D",
        "Water2D",
        "Distortion2D",
        "Fire2D",
        "Fog2D",
        "Outline2D",
        "Bloom2D",
        "GpuParticles2D",
        "DamageEffect2D",
        "PixelArtShader2D",
        "Material2D",
        "ParticleEmitter",
        "ParallaxLayer",
        "TilemapCollider",
        "ObjectiveMarker",
        "Checkpoint",
        "DontDestroyOnLoad",
        "CharacterController2D",
        "EconomyWallet",
        "Timer",
        "Tween",
    ]
}

pub fn advanced_component_category(component_type: &str) -> Option<&'static str> {
    Some(match component_type {
        "Actor2D" => "Core",
        "GameMode2D" | "GameState2D" | "PlayerState2D" | "Pawn2D" | "Controller2D"
        | "PlayerController2D" => "Gameplay",
        "AIController2D" | "BehaviorTree2D" => "AI",
        "AssetIdentity2D" => "Assets",
        "TilemapRenderer2D"
        | "Tilemap2D"
        | "TilemapChunk2D"
        | "Tileset2D"
        | "FlipbookAnimation2D" => "Paper2D",
        "AnimationBlueprint2D" | "Animator2D" | "AnimatedSprite" | "AnimationPlayer" => "Animation",
        "ScriptComponent" | "ScriptSchedule" | "VisualGraphComponent" => "Scripting",
        "AudioSource2D" => "Audio",
        "Camera2D" => "Camera",
        "Transform3D" | "MeshRenderer3D" | "Material3D" | "Billboard3D" | "HybridScene3D" => {
            "Rendering3D"
        }
        "Camera3D" => "Camera",
        "Light3D" => "Lighting3D",
        "WorldPartition2D" | "StreamingChunk2D" => "WorldStreaming",
        "RuntimeBudget2D" => "Performance",
        "ObjectPool2D" | "SpawnDirector2D" => "MassiveGameplay",
        "SaveShard2D" => "Persistence",
        "WidgetCanvas2D" => "UI",
        "Sequencer2D" => "Cinematics",
        "Area2D" | "OneWayPlatform2D" | "Trigger2D" | "StaticBody2D" | "KinematicBody2D"
        | "CharacterBody2D" => "Physics",
        "Stats"
        | "Inventory"
        | "Equipment"
        | "Ability"
        | "Interaction"
        | "Lifetime"
        | "Spawner"
        | "LootTable"
        | "Checkpoint"
        | "CharacterController2D"
        | "EconomyWallet" => "Gameplay",
        "AIController" => "AI",
        "RTSController"
        | "Commandable"
        | "SquadMember"
        | "RtsBrain"
        | "ProductionRecipeBook"
        | "Vision"
        | "FogOfWar"
        | "ThreatSource"
        | "InfluenceSource"
        | "ProductionQueue"
        | "Buildable"
        | "ConstructionSite" => "RTS",
        "NavAgent" => "Navigation",
        "DamageDealer" | "StatusEffects" | "CombatTarget" => "Combat",
        "CameraFollow" | "CameraShake" => "Camera",
        "Saveable" | "DontDestroyOnLoad" => "Persistence",
        "Blackboard" | "InputActions2D" | "EventBus2D" | "StateMachine" | "Timer" | "Tween" => {
            "Scripting"
        }
        "QuestLog" | "Dialogue" => "Narrative",
        "Light2D" | "ShadowCaster2D" | "NormalMap2D" | "Material2D" | "ParallaxLayer" => {
            "Rendering"
        }
        "Water2D" | "Distortion2D" | "Fire2D" | "Fog2D" | "Outline2D" | "Bloom2D"
        | "GpuParticles2D" | "DamageEffect2D" | "PixelArtShader2D" | "ParticleEmitter" => "Effects",
        "TilemapCollider" => "Physics",
        "ObjectiveMarker" => "UI",
        _ => return None,
    })
}
