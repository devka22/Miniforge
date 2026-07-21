//! Data-driven survival building blocks shared by projects.
//!
//! The module intentionally contains no game names, maps, balance tables or
//! art references. Designers configure ordinary components in the inspector;
//! the runtime handles needs, consumables, loot containers, recipes and
//! harvestable resources without requiring a custom Rust gameplay loop.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::component::{Component, default_component};
use crate::entities::game_object::GameObject;

const NEED_FIELDS: [&str; 9] = [
    "hunger",
    "thirst",
    "energy",
    "fatigue",
    "stamina",
    "wetness",
    "pain",
    "infection",
    "bleeding",
];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SurvivalTickReport {
    pub updated: bool,
    pub health_damage: f64,
    pub resource_respawned: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CraftResult {
    pub crafted: bool,
    pub recipe_id: String,
    pub missing_items: Vec<String>,
    pub outputs: Vec<(String, i64)>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SurvivalInteractionResult {
    pub success: bool,
    pub action: String,
    pub amount: i64,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SurvivalSystems;

impl SurvivalSystems {
    /// Advances all automatic survival state owned by one entity.
    pub fn tick_entity(entity: &mut GameObject, dt: f64) -> SurvivalTickReport {
        let dt = finite_or(dt, 0.0).clamp(0.0, 0.25);
        let mut report = SurvivalTickReport::default();

        if let Some(harvestable) = entity.get_component_mut("Harvestable")
            && harvestable.get_bool("depleted", false)
            && harvestable.get_f64("respawn_seconds", 0.0) > 0.0
        {
            let elapsed = harvestable.get_f64("respawn_elapsed", 0.0) + dt;
            let respawn_seconds = harvestable.get_f64("respawn_seconds", 0.0);
            if elapsed >= respawn_seconds {
                let max_amount = harvestable.get_f64("max_amount", 1.0).max(0.0);
                harvestable.set_f64("amount", max_amount);
                harvestable.set_f64("respawn_elapsed", 0.0);
                harvestable.set("depleted", json!(false));
                entity.enabled = true;
                entity.active = true;
                entity.visible = true;
                report.resource_respawned = true;
            } else {
                harvestable.set_f64("respawn_elapsed", elapsed);
            }
        }

        let Some(needs) = entity.get_component_mut("SurvivalNeeds") else {
            return report;
        };
        if !needs.enabled || !needs.get_bool("auto_update", true) || needs.get_bool("paused", false)
        {
            return report;
        }

        report.updated = true;
        decay_need(needs, "hunger", "hunger_decay_per_second", dt);
        decay_need(needs, "thirst", "thirst_decay_per_second", dt);
        decay_need(needs, "energy", "energy_decay_per_second", dt);
        gain_need(needs, "fatigue", "fatigue_gain_per_second", dt);
        let stamina = needs.get_f64("stamina", 100.0)
            + needs.get_f64("stamina_recovery_per_second", 7.0) * dt;
        needs.set_f64("stamina", stamina.clamp(0.0, 100.0));

        let critical_rate = needs.get_f64("critical_damage_per_second", 2.0).max(0.0);
        let mut danger = 0.0;
        if needs.get_f64("hunger", 100.0) <= 0.0 {
            danger += 1.0;
        }
        if needs.get_f64("thirst", 100.0) <= 0.0 {
            danger += 1.5;
        }
        danger += needs.get_f64("bleeding", 0.0).clamp(0.0, 100.0) / 100.0;
        danger += (needs.get_f64("infection", 0.0).clamp(0.0, 100.0) - 70.0).max(0.0) / 30.0;
        report.health_damage = critical_rate * danger * dt;
        if report.health_damage > 0.0
            && let Some(health) = entity.get_component_mut("Health")
        {
            health.take_damage(report.health_damage);
        }
        report
    }

    pub fn need(entity: &GameObject, name: &str) -> Option<f64> {
        is_need_field(name).then(|| {
            entity
                .get_component("SurvivalNeeds")
                .map(|needs| needs.get_f64(name, 0.0))
                .unwrap_or(0.0)
        })
    }

    /// Produces a UI/save-friendly view model from built-in components.
    pub fn state(entity: &GameObject) -> Value {
        let health = entity.get_component("Health");
        let current_health = health
            .map(|value| value.get_f64("health", 0.0))
            .unwrap_or(0.0);
        let max_health = health
            .map(|value| value.get_f64("max_health", 100.0))
            .unwrap_or(100.0)
            .max(0.0001);
        let needs = entity
            .get_component("SurvivalNeeds")
            .map(|component| {
                NEED_FIELDS
                    .into_iter()
                    .map(|field| (field.to_string(), json!(component.get_f64(field, 0.0))))
                    .collect::<serde_json::Map<_, _>>()
            })
            .unwrap_or_default();
        let inventory = entity.get_component("Inventory");
        json!({
            "player": {
                "health": {
                    "value": current_health,
                    "max": max_health,
                    "percent": current_health / max_health,
                },
                "needs": needs,
                "inventory": {
                    "items": inventory.and_then(|value| value.get("items")).cloned().unwrap_or_else(|| json!([])),
                    "slots_used": inventory.and_then(|value| value.get("items")).and_then(Value::as_array).map(Vec::len).unwrap_or(0),
                    "capacity": inventory.map(|value| value.get_i64("capacity", 0)).unwrap_or(0),
                    "weight": inventory.map(Component::inventory_weight).unwrap_or(0.0),
                    "max_weight": inventory.map(|value| value.get_f64("max_weight", 0.0)).unwrap_or(0.0),
                }
            }
        })
    }

    pub fn set_need(entity: &mut GameObject, name: &str, value: f64) -> bool {
        if !is_need_field(name) {
            return false;
        }
        ensure_component(entity, "SurvivalNeeds");
        let Some(needs) = entity.get_component_mut("SurvivalNeeds") else {
            return false;
        };
        needs.set_f64(name, finite_or(value, 0.0).clamp(0.0, 100.0));
        true
    }

    pub fn modify_need(entity: &mut GameObject, name: &str, delta: f64) -> bool {
        let current = Self::need(entity, name).unwrap_or(0.0);
        Self::set_need(entity, name, current + finite_or(delta, 0.0))
    }

    /// Applies `metadata.effects` from one inventory item and consumes it.
    /// Supported effect keys are `health` and every `SurvivalNeeds` field.
    pub fn use_item(entity: &mut GameObject, item_id: &str) -> bool {
        let Some(metadata) = inventory_item_metadata(entity, item_id) else {
            return false;
        };
        let Some(effects) = metadata.get("effects").and_then(Value::as_object) else {
            return false;
        };
        if effects.is_empty() {
            return false;
        }
        if entity
            .get_component_mut("Inventory")
            .map(|inventory| inventory.inventory_remove_item(item_id, 1))
            .unwrap_or(0)
            != 1
        {
            return false;
        }

        for (name, value) in effects {
            let amount = value.as_f64().unwrap_or(0.0);
            if name == "health" {
                ensure_component(entity, "Health");
                if let Some(health) = entity.get_component_mut("Health") {
                    if amount >= 0.0 {
                        health.heal(amount);
                    } else {
                        health.take_damage(-amount);
                    }
                }
            } else {
                let _ = Self::modify_need(entity, name, amount);
            }
        }
        true
    }

    /// Materializes configured loot exactly once. Empty tables are valid.
    pub fn search_container(container: &mut GameObject) -> usize {
        fill_container_once(container, "loot_entries", "rolls", "searched", 0xA11C_E001)
    }

    /// Performs the optional second hidden-loot roll exactly once.
    pub fn rummage_container(container: &mut GameObject) -> usize {
        if container
            .get_component("LootContainer")
            .is_some_and(|component| !component.get_bool("searched", false))
        {
            let _ = Self::search_container(container);
        }
        fill_container_once(
            container,
            "hidden_entries",
            "hidden_rolls",
            "rummaged",
            0xBADC_0FFE,
        )
    }

    pub fn container_items(container: &GameObject) -> Vec<Value> {
        container
            .get_component("LootContainer")
            .and_then(|component| component.get("items"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    pub fn take_from_container(
        container: &mut GameObject,
        actor: &mut GameObject,
        item_id: &str,
        quantity: i64,
    ) -> i64 {
        let Some(component) = container.get_component("LootContainer") else {
            return 0;
        };
        if component.get_bool("locked", false) {
            return 0;
        }
        let mut items = component
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let Some(index) = items
            .iter()
            .position(|item| item.get("id").and_then(Value::as_str) == Some(item_id))
        else {
            return 0;
        };
        let available = items[index]
            .get("quantity")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0);
        let requested = quantity.max(0).min(available);
        let metadata = items[index]
            .get("metadata")
            .cloned()
            .unwrap_or_else(|| json!({}));
        ensure_component(actor, "Inventory");
        let moved = actor
            .get_component_mut("Inventory")
            .map(|inventory| inventory.inventory_add_item(item_id, requested, metadata))
            .unwrap_or(0);
        if moved <= 0 {
            return 0;
        }
        let remaining = available - moved;
        if remaining <= 0 {
            items.remove(index);
        } else if let Some(item) = items[index].as_object_mut() {
            item.insert("quantity".to_string(), json!(remaining));
        }
        if let Some(component) = container.get_component_mut("LootContainer") {
            component.set("items", Value::Array(items));
        }
        moved
    }

    pub fn take_all(container: &mut GameObject, actor: &mut GameObject) -> i64 {
        let mut moved = 0;
        for item in Self::container_items(container) {
            let Some(item_id) = item.get("id").and_then(Value::as_str) else {
                continue;
            };
            let quantity = item.get("quantity").and_then(Value::as_i64).unwrap_or(0);
            moved += Self::take_from_container(container, actor, item_id, quantity);
        }
        moved
    }

    pub fn can_craft(actor: &GameObject, recipe_id: &str) -> bool {
        let Some(book) = actor.get_component("CraftingBook") else {
            return false;
        };
        recipe_is_available(book, recipe_id)
            && recipe_by_id(book, recipe_id)
                .is_some_and(|recipe| preview_craft(actor, &recipe).crafted)
    }

    pub fn craft(actor: &mut GameObject, recipe_id: &str) -> CraftResult {
        let Some(book) = actor.get_component("CraftingBook").cloned() else {
            return failed_craft(recipe_id, "recipe_book");
        };
        if !recipe_is_available(&book, recipe_id) {
            return failed_craft(recipe_id, "recipe_locked");
        }
        let Some(recipe) = recipe_by_id(&book, recipe_id) else {
            return failed_craft(recipe_id, "recipe_missing");
        };
        apply_recipe(actor, recipe_id, &recipe)
    }

    pub fn craft_at(actor: &mut GameObject, station: &GameObject, recipe_id: &str) -> CraftResult {
        let Some(component) = station.get_component("CraftingStation") else {
            return failed_craft(recipe_id, "station_missing");
        };
        if !component.get_bool("powered", true) {
            return failed_craft(recipe_id, "station_unpowered");
        }
        let Some(recipe) = recipe_by_id(component, recipe_id) else {
            return failed_craft(recipe_id, "recipe_missing");
        };
        apply_recipe(actor, recipe_id, &recipe)
    }

    pub fn harvest(source: &mut GameObject, actor: &mut GameObject) -> i64 {
        let Some(harvestable) = source.get_component("Harvestable").cloned() else {
            return 0;
        };
        if harvestable.get_bool("depleted", false) {
            return 0;
        }
        let required_tool = harvestable
            .get("required_tool")
            .and_then(Value::as_str)
            .filter(|tool| !tool.is_empty());
        if required_tool.is_some_and(|tool| inventory_count(actor, tool) <= 0) {
            return 0;
        }
        let remaining = harvestable.get_f64("amount", 0.0).max(0.0);
        if remaining <= 0.0 {
            mark_depleted(source);
            return 0;
        }
        let multiplier = if required_tool.is_some() {
            harvestable.get_f64("tool_multiplier", 1.0).max(0.0)
        } else {
            1.0
        };
        let requested = ((harvestable.get_i64("yield_per_action", 1).max(1) as f64 * multiplier)
            .floor() as i64)
            .max(1)
            .min(remaining.floor().max(1.0) as i64);
        let item_id = harvestable.get_string("item_id", "resource");
        let metadata = harvestable
            .get("item_metadata")
            .cloned()
            .unwrap_or_else(|| json!({}));
        ensure_component(actor, "Inventory");
        let added = actor
            .get_component_mut("Inventory")
            .map(|inventory| inventory.inventory_add_item(&item_id, requested, metadata))
            .unwrap_or(0);
        if added <= 0 {
            return 0;
        }
        if let Some(harvestable) = source.get_component_mut("Harvestable") {
            let next = (remaining - added as f64).max(0.0);
            harvestable.set_f64("amount", next);
            if next <= 0.0 {
                harvestable.set("depleted", json!(true));
                harvestable.set_f64("respawn_elapsed", 0.0);
            }
        }
        if source
            .get_component("Harvestable")
            .is_some_and(|component| component.get_bool("depleted", false))
        {
            mark_depleted(source);
        }
        added
    }

    /// Resolves the common built-in interaction types with one engine call.
    pub fn interact(actor: &mut GameObject, target: &mut GameObject) -> SurvivalInteractionResult {
        if target.get_component("LootContainer").is_some() {
            if target
                .get_component("LootContainer")
                .is_some_and(|component| component.get_bool("locked", false))
            {
                return interaction(false, "loot", 0, "container_locked");
            }
            let amount = Self::search_container(target) as i64;
            return interaction(true, "loot", amount, "container_opened");
        }
        if target.get_component("Harvestable").is_some() {
            let amount = Self::harvest(target, actor);
            return interaction(amount > 0, "harvest", amount, "resource_harvested");
        }
        if target.get_component("CraftingStation").is_some() {
            return interaction(true, "craft", 0, "station_opened");
        }
        if let Some((item_id, quantity, metadata)) = pickup_data(target) {
            ensure_component(actor, "Inventory");
            let added = actor
                .get_component_mut("Inventory")
                .map(|inventory| inventory.inventory_add_item(&item_id, quantity, metadata))
                .unwrap_or(0);
            if added > 0 {
                let remaining = quantity - added;
                if remaining <= 0 {
                    target.enabled = false;
                    target.active = false;
                    target.visible = false;
                } else if let Some(blackboard) = target.get_component_mut("Blackboard") {
                    blackboard.blackboard_set("quantity", json!(remaining));
                }
            }
            return interaction(added > 0, "pickup", added, "item_picked_up");
        }
        interaction(false, "none", 0, "unsupported_target")
    }
}

fn decay_need(component: &mut Component, field: &str, rate_field: &str, dt: f64) {
    let value = component.get_f64(field, 100.0) - component.get_f64(rate_field, 0.0).max(0.0) * dt;
    component.set_f64(field, value.clamp(0.0, 100.0));
}

fn gain_need(component: &mut Component, field: &str, rate_field: &str, dt: f64) {
    let value = component.get_f64(field, 0.0) + component.get_f64(rate_field, 0.0).max(0.0) * dt;
    component.set_f64(field, value.clamp(0.0, 100.0));
}

fn is_need_field(name: &str) -> bool {
    NEED_FIELDS.contains(&name)
}

fn ensure_component(entity: &mut GameObject, component_type: &str) {
    if entity.get_component(component_type).is_none()
        && let Some(component) = default_component(component_type)
    {
        entity.add_component(component);
    }
}

fn inventory_count(entity: &GameObject, item_id: &str) -> i64 {
    entity
        .get_component("Inventory")
        .map(|inventory| inventory.inventory_count_item(item_id))
        .unwrap_or(0)
}

fn inventory_item_metadata(entity: &GameObject, item_id: &str) -> Option<Value> {
    entity
        .get_component("Inventory")?
        .get("items")?
        .as_array()?
        .iter()
        .find(|item| item.get("id").and_then(Value::as_str) == Some(item_id))?
        .get("metadata")
        .cloned()
}

fn fill_container_once(
    entity: &mut GameObject,
    entries_key: &str,
    rolls_key: &str,
    state_key: &str,
    salt: u64,
) -> usize {
    let Some(component) = entity.get_component("LootContainer").cloned() else {
        return 0;
    };
    if component.get_bool("locked", false) || component.get_bool(state_key, false) {
        return 0;
    }
    let entries = component
        .get(entries_key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let rolls = component.get_i64(rolls_key, 1).max(0) as usize;
    let capacity = component.get_i64("capacity", 18).max(0) as usize;
    let stack_limit = component.get_i64("stack_limit", 99).max(1);
    let mut items = component
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut generated = 0;
    for roll in 0..rolls {
        let seed = entity.id ^ salt ^ (roll as u64).wrapping_mul(0x9E37_79B9);
        let Some(entry) = choose_weighted(&entries, seed) else {
            continue;
        };
        let item_id = entry.get("id").and_then(Value::as_str).unwrap_or("item");
        let min = entry.get("min").and_then(Value::as_i64).unwrap_or(1).max(0);
        let max = entry
            .get("max")
            .and_then(Value::as_i64)
            .unwrap_or(min)
            .max(min);
        let quantity =
            min + (stable_hash(seed ^ 0xD00D_F00D) % (max.saturating_sub(min) as u64 + 1)) as i64;
        let metadata = entry.get("metadata").cloned().unwrap_or_else(|| json!({}));
        generated += add_to_stacks(
            &mut items,
            capacity,
            stack_limit,
            item_id,
            quantity,
            metadata,
        ) as usize;
    }
    if let Some(component) = entity.get_component_mut("LootContainer") {
        component.set("items", Value::Array(items));
        component.set(state_key, json!(true));
    }
    generated
}

fn add_to_stacks(
    items: &mut Vec<Value>,
    capacity: usize,
    default_stack_limit: i64,
    item_id: &str,
    mut quantity: i64,
    metadata: Value,
) -> i64 {
    quantity = quantity.max(0);
    let requested = quantity;
    for item in items.iter_mut() {
        if quantity <= 0 || item.get("id").and_then(Value::as_str) != Some(item_id) {
            continue;
        }
        let limit = item
            .get("stack_limit")
            .and_then(Value::as_i64)
            .unwrap_or(default_stack_limit)
            .max(1);
        let current = item
            .get("quantity")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0);
        let moved = (limit - current).max(0).min(quantity);
        if let Some(map) = item.as_object_mut() {
            map.insert("quantity".to_string(), json!(current + moved));
        }
        quantity -= moved;
    }
    while quantity > 0 && items.len() < capacity {
        let moved = quantity.min(default_stack_limit.max(1));
        items.push(json!({
            "id": item_id,
            "quantity": moved,
            "stack_limit": default_stack_limit.max(1),
            "metadata": metadata,
        }));
        quantity -= moved;
    }
    requested - quantity
}

fn choose_weighted(entries: &[Value], seed: u64) -> Option<&Value> {
    let total = entries
        .iter()
        .map(|entry| {
            entry
                .get("weight")
                .and_then(Value::as_f64)
                .unwrap_or(1.0)
                .max(0.0)
        })
        .sum::<f64>();
    if total <= f64::EPSILON {
        return entries.first();
    }
    let mut cursor = stable_unit(seed) * total;
    for entry in entries {
        cursor -= entry
            .get("weight")
            .and_then(Value::as_f64)
            .unwrap_or(1.0)
            .max(0.0);
        if cursor <= 0.0 {
            return Some(entry);
        }
    }
    entries.last()
}

fn stable_hash(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn stable_unit(seed: u64) -> f64 {
    (stable_hash(seed) >> 11) as f64 / ((1u64 << 53) as f64)
}

fn recipe_by_id(component: &Component, recipe_id: &str) -> Option<Value> {
    component
        .get("recipes")?
        .as_array()?
        .iter()
        .find(|recipe| recipe.get("id").and_then(Value::as_str) == Some(recipe_id))
        .cloned()
}

fn recipe_is_available(book: &Component, recipe_id: &str) -> bool {
    book.get_bool("allow_all_recipes", true)
        || book
            .get_string_list("known_recipes")
            .iter()
            .any(|known| known == recipe_id)
}

fn preview_craft(actor: &GameObject, recipe: &Value) -> CraftResult {
    let recipe_id = recipe
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("recipe")
        .to_string();
    let mut inventory = actor
        .get_component("Inventory")
        .cloned()
        .or_else(|| default_component("Inventory"))
        .expect("Inventory component is built in");
    let mut result = CraftResult {
        recipe_id,
        ..CraftResult::default()
    };
    for ingredient in value_array(recipe, "ingredients") {
        let item_id = ingredient.get("id").and_then(Value::as_str).unwrap_or("");
        let quantity = ingredient
            .get("quantity")
            .and_then(Value::as_i64)
            .unwrap_or(1)
            .max(1);
        if inventory.inventory_count_item(item_id) < quantity {
            result.missing_items.push(item_id.to_string());
        } else {
            inventory.inventory_remove_item(item_id, quantity);
        }
    }
    if !result.missing_items.is_empty() {
        return result;
    }
    for output in value_array(recipe, "outputs") {
        let item_id = output.get("id").and_then(Value::as_str).unwrap_or("item");
        let quantity = output
            .get("quantity")
            .and_then(Value::as_i64)
            .unwrap_or(1)
            .max(1);
        let metadata = output.get("metadata").cloned().unwrap_or_else(|| json!({}));
        if inventory.inventory_add_item(item_id, quantity, metadata) != quantity {
            result.missing_items.push("inventory_capacity".to_string());
            return result;
        }
        result.outputs.push((item_id.to_string(), quantity));
    }
    result.crafted = true;
    result
}

fn apply_recipe(actor: &mut GameObject, recipe_id: &str, recipe: &Value) -> CraftResult {
    let result = preview_craft(actor, recipe);
    if !result.crafted {
        return result;
    }
    ensure_component(actor, "Inventory");
    let mut inventory = actor
        .get_component("Inventory")
        .cloned()
        .expect("inventory was ensured");
    for ingredient in value_array(recipe, "ingredients") {
        let item_id = ingredient.get("id").and_then(Value::as_str).unwrap_or("");
        let quantity = ingredient
            .get("quantity")
            .and_then(Value::as_i64)
            .unwrap_or(1)
            .max(1);
        inventory.inventory_remove_item(item_id, quantity);
    }
    for output in value_array(recipe, "outputs") {
        let item_id = output.get("id").and_then(Value::as_str).unwrap_or("item");
        let quantity = output
            .get("quantity")
            .and_then(Value::as_i64)
            .unwrap_or(1)
            .max(1);
        let metadata = output.get("metadata").cloned().unwrap_or_else(|| json!({}));
        let added = inventory.inventory_add_item(item_id, quantity, metadata);
        debug_assert_eq!(added, quantity, "preview and commit must match");
    }
    if let Some(current) = actor.get_component_mut("Inventory") {
        *current = inventory;
    }
    CraftResult {
        recipe_id: recipe_id.to_string(),
        ..result
    }
}

fn value_array(value: &Value, key: &str) -> Vec<Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn failed_craft(recipe_id: &str, reason: &str) -> CraftResult {
    CraftResult {
        recipe_id: recipe_id.to_string(),
        missing_items: vec![reason.to_string()],
        ..CraftResult::default()
    }
}

fn mark_depleted(source: &mut GameObject) {
    let Some(harvestable) = source.get_component("Harvestable") else {
        return;
    };
    if harvestable.get_bool("destroy_when_depleted", false)
        && harvestable.get_f64("respawn_seconds", 0.0) <= 0.0
    {
        source.enabled = false;
        source.active = false;
        source.visible = false;
    }
}

fn pickup_data(target: &GameObject) -> Option<(String, i64, Value)> {
    let blackboard = target.get_component("Blackboard")?;
    let item_id = blackboard
        .blackboard_get("item_id", Value::Null)
        .as_str()?
        .to_string();
    let quantity = blackboard
        .blackboard_get("quantity", json!(1))
        .as_i64()
        .unwrap_or(1)
        .max(1);
    let metadata = blackboard.blackboard_get("metadata", json!({}));
    Some((item_id, quantity, metadata))
}

fn interaction(
    success: bool,
    action: &str,
    amount: i64,
    message: &str,
) -> SurvivalInteractionResult {
    SurvivalInteractionResult {
        success,
        action: action.to_string(),
        amount,
        message: message.to_string(),
    }
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::archetype_library::ArchetypeLibrary;
    use crate::engine::component_registry::ComponentRegistry;
    use crate::engine::miniforge_2d::ui_framework::survival_hud_canvas;
    use crate::engine::project_templates::ProjectTemplates;
    use crate::engine::visual_scripting::VisualScriptRuntime;

    fn actor() -> GameObject {
        let mut actor = GameObject::new_unit(0.0, 0.0, Some("Actor".to_string()));
        actor.add_component(default_component("Health").unwrap());
        actor.add_component(default_component("SurvivalNeeds").unwrap());
        actor.add_component(default_component("Inventory").unwrap());
        actor
    }

    #[test]
    fn needs_tick_automatically_and_critical_needs_damage_health() {
        let mut actor = actor();
        let needs = actor.get_component_mut("SurvivalNeeds").unwrap();
        needs.set_f64("hunger", 0.0);
        needs.set_f64("thirst", 0.0);
        needs.set_f64("critical_damage_per_second", 4.0);

        let report = SurvivalSystems::tick_entity(&mut actor, 0.25);

        assert!(report.updated);
        assert!(report.health_damage > 0.0);
        assert!(
            actor
                .get_component("Health")
                .unwrap()
                .get_f64("health", 0.0)
                < 100.0
        );
    }

    #[test]
    fn consumables_apply_data_driven_effects_without_game_code() {
        let mut actor = actor();
        SurvivalSystems::set_need(&mut actor, "thirst", 20.0);
        actor
            .get_component_mut("Health")
            .unwrap()
            .set_f64("health", 50.0);
        actor
            .get_component_mut("Inventory")
            .unwrap()
            .inventory_add_item(
                "drink",
                2,
                json!({"weight": 0.5, "effects": {"thirst": 35.0, "health": 5.0}}),
            );

        assert!(SurvivalSystems::use_item(&mut actor, "drink"));
        assert_eq!(SurvivalSystems::need(&actor, "thirst"), Some(55.0));
        assert_eq!(
            actor
                .get_component("Health")
                .unwrap()
                .get_f64("health", 0.0),
            55.0
        );
        assert_eq!(inventory_count(&actor, "drink"), 1);
    }

    #[test]
    fn inventory_weight_limits_and_sorting_are_native() {
        let mut actor = actor();
        let inventory = actor.get_component_mut("Inventory").unwrap();
        inventory.set_f64("max_weight", 1.0);
        assert_eq!(
            inventory.inventory_add_item("heavy", 3, json!({"weight": 0.5, "category": "tools"}),),
            2
        );
        assert_eq!(inventory.inventory_weight(), 1.0);
        inventory.inventory_sort_items("category");
        assert_eq!(inventory.get_string("sort_mode", ""), "category");
    }

    #[test]
    fn searchable_and_hidden_loot_transfer_persistently() {
        let mut container = GameObject::new(1.0, 0.0, Some("Container".to_string()));
        let mut loot = default_component("LootContainer").unwrap();
        loot.set(
            "loot_entries",
            json!([{"id": "cloth", "weight": 1.0, "min": 2, "max": 2}]),
        );
        loot.set(
            "hidden_entries",
            json!([{"id": "tool", "weight": 1.0, "min": 1, "max": 1}]),
        );
        container.add_component(loot);
        let mut actor = actor();

        assert_eq!(SurvivalSystems::search_container(&mut container), 2);
        assert_eq!(SurvivalSystems::search_container(&mut container), 0);
        assert_eq!(SurvivalSystems::rummage_container(&mut container), 1);
        assert_eq!(SurvivalSystems::take_all(&mut container, &mut actor), 3);
        assert!(SurvivalSystems::container_items(&container).is_empty());
        assert_eq!(inventory_count(&actor, "cloth"), 2);
        assert_eq!(inventory_count(&actor, "tool"), 1);
        assert!(
            container
                .get_component("LootContainer")
                .unwrap()
                .get_bool("rummaged", false)
        );
    }

    #[test]
    fn crafting_is_atomic_and_configured_only_with_recipe_data() {
        let mut actor = actor();
        actor
            .get_component_mut("Inventory")
            .unwrap()
            .inventory_add_item("fiber", 2, json!({}));
        let mut book = default_component("CraftingBook").unwrap();
        book.set(
            "recipes",
            json!([{
                "id": "basic_recipe",
                "ingredients": [{"id": "fiber", "quantity": 2}],
                "outputs": [{"id": "crafted_item", "quantity": 1, "metadata": {"weight": 0.2}}]
            }]),
        );
        actor.add_component(book);

        assert!(SurvivalSystems::can_craft(&actor, "basic_recipe"));
        let result = SurvivalSystems::craft(&mut actor, "basic_recipe");
        assert!(result.crafted);
        assert_eq!(inventory_count(&actor, "fiber"), 0);
        assert_eq!(inventory_count(&actor, "crafted_item"), 1);
        assert!(!SurvivalSystems::craft(&mut actor, "basic_recipe").crafted);
    }

    #[test]
    fn harvesting_enforces_tools_inventory_and_respawn() {
        let mut actor = actor();
        actor
            .get_component_mut("Inventory")
            .unwrap()
            .inventory_add_item("tool", 1, json!({}));
        let mut source = GameObject::new(1.0, 0.0, Some("Resource".to_string()));
        let mut harvestable = default_component("Harvestable").unwrap();
        harvestable.set("item_id", json!("material"));
        harvestable.set_f64("amount", 2.0);
        harvestable.set_f64("max_amount", 2.0);
        harvestable.set("required_tool", json!("tool"));
        harvestable.set("yield_per_action", json!(2));
        harvestable.set_f64("respawn_seconds", 1.0);
        source.add_component(harvestable);

        assert_eq!(SurvivalSystems::harvest(&mut source, &mut actor), 2);
        assert_eq!(inventory_count(&actor, "material"), 2);
        assert!(
            source
                .get_component("Harvestable")
                .unwrap()
                .get_bool("depleted", false)
        );
        let mut respawned = false;
        for _ in 0..4 {
            respawned |= SurvivalSystems::tick_entity(&mut source, 0.25).resource_respawned;
        }
        assert!(respawned);
        assert_eq!(
            source
                .get_component("Harvestable")
                .unwrap()
                .get_f64("amount", 0.0),
            2.0
        );
    }

    #[test]
    fn survival_components_and_archetypes_are_available_without_project_code() {
        let registry = ComponentRegistry::new();
        for component in [
            "SurvivalNeeds",
            "SurvivalUIBinding",
            "LootContainer",
            "CraftingBook",
            "CraftingStation",
            "Harvestable",
        ] {
            assert!(registry.create(component).is_some(), "missing {component}");
            assert_eq!(registry.category_for(component), Some("Survival"));
        }

        let library = ArchetypeLibrary::with_defaults();
        let actor = library
            .instantiate("survival_actor", 0.0, 0.0, None)
            .expect("survival actor preset");
        assert!(actor.get_component("Health").is_some());
        assert!(actor.get_component("SurvivalNeeds").is_some());
        assert!(actor.get_component("Inventory").is_some());
        assert!(actor.get_component("CraftingBook").is_some());

        let state = SurvivalSystems::state(&actor);
        assert_eq!(state["player"]["health"]["percent"], json!(1.0));
        assert_eq!(state["player"]["needs"]["hunger"], json!(100.0));
        let bindings = survival_hud_canvas().binding_paths();
        assert!(bindings.contains(&"player.health.percent".to_string()));
        assert!(bindings.contains(&"player.needs.thirst".to_string()));
    }

    #[test]
    fn visual_script_can_change_needs_and_use_items_without_custom_code() {
        let mut actor = actor();
        actor
            .get_component_mut("Inventory")
            .unwrap()
            .inventory_add_item("consumable", 1, json!({"effects": {"thirst": 25.0}}));
        SurvivalSystems::set_need(&mut actor, "thirst", 10.0);
        let mut visual = default_component("VisualScript").unwrap();
        visual.set(
            "nodes",
            json!([
                {"id": "start", "type": "EventStart", "next": "use"},
                {"id": "use", "type": "UseInventoryItem", "item": "consumable", "next": "fatigue"},
                {"id": "fatigue", "type": "ModifySurvivalNeed", "need": "fatigue", "delta": 5.0}
            ]),
        );
        actor.add_component(visual);
        let mut entities = vec![actor];

        VisualScriptRuntime::default().update_entities(&mut entities, 0.016, "PLAY");

        assert_eq!(SurvivalSystems::need(&entities[0], "thirst"), Some(35.0));
        assert_eq!(SurvivalSystems::need(&entities[0], "fatigue"), Some(5.0));
        assert_eq!(inventory_count(&entities[0], "consumable"), 0);
    }

    #[test]
    fn survival_project_template_contains_native_components_not_game_content() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "miniforge_survival_template_{}_{}",
            std::process::id(),
            unique
        ));
        let created = ProjectTemplates::create(&root, "Survival").unwrap();
        let scene_path = created
            .iter()
            .find(|path| path.extension().and_then(|value| value.to_str()) == Some("scene"))
            .unwrap();
        let scene: Value = serde_json::from_slice(&std::fs::read(scene_path).unwrap()).unwrap();

        assert_eq!(scene["settings"]["contains_game_content"], json!(false));
        let entities = scene["entities"].as_array().unwrap();
        assert_eq!(entities.len(), 9);
        assert_eq!(scene["ui_canvases"].as_array().unwrap().len(), 0);
        assert_eq!(
            entities
                .iter()
                .filter(
                    |entity| entity["components"]
                        .as_array()
                        .is_some_and(|components| components.iter().any(|component| {
                            component["component_type"] == json!("SurvivalUIBinding")
                        }))
                )
                .count(),
            5
        );
        let actor_prefab_path = created
            .iter()
            .find(|path| {
                path.file_name().and_then(|value| value.to_str()) == Some("SurvivalActor.prefab")
            })
            .unwrap();
        let actor_prefab: Value =
            serde_json::from_slice(&std::fs::read(actor_prefab_path).unwrap()).unwrap();
        assert!(
            actor_prefab["entity"]["components"]
                .as_array()
                .unwrap()
                .iter()
                .any(|component| component["component_type"] == json!("Inventory"))
        );
        assert!(created.iter().all(|path| path.starts_with(&root)));

        std::fs::remove_dir_all(root).unwrap();
    }
}
