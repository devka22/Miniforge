//! Data-driven survival building blocks shared by projects.
//!
//! The module intentionally contains no game names, maps, balance tables or
//! art references. Designers configure ordinary components in the inspector;
//! the runtime handles needs, consumables, loot containers, recipes and
//! harvestable resources without requiring a custom Rust gameplay loop.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::component::{Component, default_component};
use crate::entities::game_object::GameObject;

const NEED_FIELDS: [&str; 14] = [
    "hunger",
    "thirst",
    "energy",
    "fatigue",
    "stamina",
    "wetness",
    "pain",
    "infection",
    "bleeding",
    "stress",
    "morale",
    "hygiene",
    "sickness",
    "oxygen",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurvivalEnvironment2D {
    pub ambient_temperature_c: f64,
    pub wind_speed: f64,
    pub precipitation: f64,
    pub shelter: f64,
    pub heat_source: f64,
    pub exertion: f64,
    pub air_quality: f64,
    pub pathogen_exposure: f64,
    pub daylight: f64,
}

impl Default for SurvivalEnvironment2D {
    fn default() -> Self {
        Self {
            ambient_temperature_c: 20.0,
            wind_speed: 0.0,
            precipitation: 0.0,
            shelter: 0.0,
            heat_source: 0.0,
            exertion: 0.0,
            air_quality: 1.0,
            pathogen_exposure: 0.0,
            daylight: 1.0,
        }
    }
}

impl SurvivalEnvironment2D {
    pub fn from_component(component: &Component) -> Self {
        Self {
            ambient_temperature_c: finite_or(
                component.get_f64("ambient_temperature_c", 20.0),
                20.0,
            )
            .clamp(-100.0, 100.0),
            wind_speed: finite_or(component.get_f64("wind_speed", 0.0), 0.0).clamp(0.0, 250.0),
            precipitation: finite_or(component.get_f64("precipitation", 0.0), 0.0).clamp(0.0, 1.0),
            shelter: finite_or(component.get_f64("shelter", 0.0), 0.0).clamp(0.0, 1.0),
            heat_source: finite_or(component.get_f64("heat_source", 0.0), 0.0).clamp(0.0, 1.0),
            exertion: finite_or(component.get_f64("exertion", 0.0), 0.0).clamp(0.0, 1.0),
            air_quality: finite_or(component.get_f64("air_quality", 1.0), 1.0).clamp(0.0, 1.0),
            pathogen_exposure: finite_or(component.get_f64("pathogen_exposure", 0.0), 0.0)
                .clamp(0.0, 1.0),
            daylight: finite_or(component.get_f64("daylight", 1.0), 1.0).clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentSummary {
    pub equipped_count: usize,
    pub item_ids: Vec<String>,
    pub occupied_slots: Vec<String>,
    pub stat_bonuses: BTreeMap<String, f64>,
    pub protection: BTreeMap<String, f64>,
    pub insulation: f64,
    pub waterproofing: f64,
    pub carry_capacity_bonus: f64,
    pub noise: f64,
    pub total_weight: f64,
    pub movement_multiplier: f64,
}

impl Default for EquipmentSummary {
    fn default() -> Self {
        Self {
            equipped_count: 0,
            item_ids: Vec::new(),
            occupied_slots: Vec::new(),
            stat_bonuses: BTreeMap::new(),
            protection: BTreeMap::new(),
            insulation: 0.0,
            waterproofing: 0.0,
            carry_capacity_bonus: 0.0,
            noise: 0.0,
            total_weight: 0.0,
            movement_multiplier: 1.0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EquipmentChangeResult {
    pub success: bool,
    pub slot: String,
    pub item_id: String,
    pub occupied_slots: Vec<String>,
    pub displaced_item_ids: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InjuryResult {
    pub success: bool,
    pub injury_id: u64,
    pub region: String,
    pub injury_type: String,
    pub severity: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SurvivalTickReport {
    pub updated: bool,
    pub health_damage: f64,
    pub resource_respawned: bool,
    pub core_temperature_c: f64,
    pub thermal_stress: f64,
    pub active_injuries: usize,
    pub equipment_weight: f64,
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
        let environment = entity
            .get_component("SurvivalEnvironment2D")
            .filter(|component| component.enabled && component.get_bool("enabled", true))
            .map(SurvivalEnvironment2D::from_component)
            .unwrap_or_default();
        Self::tick_entity_in_environment(entity, dt, &environment)
    }

    /// Advances survival state with an explicit weather/exposure sample.
    /// Games can keep weather on a world entity and pass the resolved sample
    /// here, while simple scenes may attach `SurvivalEnvironment2D` directly.
    pub fn tick_entity_in_environment(
        entity: &mut GameObject,
        dt: f64,
        environment: &SurvivalEnvironment2D,
    ) -> SurvivalTickReport {
        let dt = finite_or(dt, 0.0).clamp(0.0, 0.25);
        let mut report = SurvivalTickReport::default();
        let equipment = Self::equipment_summary(entity);
        report.equipment_weight = equipment.total_weight;

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

        let injury_progress = progress_body_condition(entity, dt, environment);
        report.active_injuries = injury_progress.active_injuries;

        let Some(needs) = entity.get_component_mut("SurvivalNeeds") else {
            report.health_damage = injury_progress.health_damage;
            if report.health_damage > 0.0
                && let Some(health) = entity.get_component_mut("Health")
            {
                health.take_damage(report.health_damage);
            }
            return report;
        };
        if !needs.enabled || !needs.get_bool("auto_update", true) || needs.get_bool("paused", false)
        {
            report.health_damage = injury_progress.health_damage;
            if report.health_damage > 0.0
                && let Some(health) = entity.get_component_mut("Health")
            {
                health.take_damage(report.health_damage);
            }
            return report;
        }

        report.updated = true;
        let cold_load = ((5.0 - environment.ambient_temperature_c) / 35.0).clamp(0.0, 1.0);
        let heat_load = ((environment.ambient_temperature_c - 28.0) / 32.0).clamp(0.0, 1.0);
        let activity = environment.exertion.clamp(0.0, 1.0);
        let encumbrance = (equipment.total_weight / 35.0).clamp(0.0, 2.0);
        decay_need_scaled(
            needs,
            "hunger",
            "hunger_decay_per_second",
            dt,
            1.0 + activity * 0.85 + cold_load * 0.45,
        );
        decay_need_scaled(
            needs,
            "thirst",
            "thirst_decay_per_second",
            dt,
            1.0 + activity + heat_load * 1.4,
        );
        decay_need(needs, "energy", "energy_decay_per_second", dt);
        gain_need(needs, "fatigue", "fatigue_gain_per_second", dt);
        let stamina_delta = if activity > 0.05 {
            -(4.0 + activity * 10.0) * (1.0 + encumbrance * 0.35)
        } else {
            needs.get_f64("stamina_recovery_per_second", 7.0)
                * equipment.movement_multiplier.clamp(0.1, 2.0)
        };
        let stamina = needs.get_f64("stamina", 100.0) + stamina_delta * dt;
        needs.set_f64("stamina", stamina.clamp(0.0, 100.0));

        let exposure = (environment.precipitation
            * (1.0 - environment.shelter)
            * (1.0 - equipment.waterproofing))
            .clamp(0.0, 1.0);
        let drying = (environment.heat_source * 4.0
            + (1.0 - environment.precipitation) * (0.15 + environment.shelter * 0.35))
            * dt;
        let wetness =
            (needs.get_f64("wetness", 0.0) + exposure * 7.0 * dt - drying).clamp(0.0, 100.0);
        needs.set_f64("wetness", wetness);

        let wind_chill = environment.wind_speed * 0.035 * (1.0 - environment.shelter);
        let wet_chill = wetness / 100.0 * 4.5 * (1.0 - equipment.waterproofing * 0.7);
        let insulation = equipment.insulation.clamp(0.0, 1.0);
        let effective_ambient = environment.ambient_temperature_c - wind_chill - wet_chill
            + environment.heat_source * 22.0;
        let thermal_offset = ((effective_ambient - 20.0) * 0.045 * (1.0 - insulation * 0.82)
            + activity * 0.65)
            .clamp(-5.0, 4.0);
        let target_temperature = 36.8 + thermal_offset;
        let current_temperature = needs.get_f64("body_temperature", 36.8);
        let temperature_rate = (0.025 + wetness / 100.0 * 0.045) * dt;
        let body_temperature = current_temperature
            + (target_temperature - current_temperature) * temperature_rate.clamp(0.0, 1.0);
        needs.set_f64("body_temperature", body_temperature.clamp(28.0, 44.0));
        report.core_temperature_c = body_temperature;
        report.thermal_stress = if body_temperature < 35.0 {
            (35.0 - body_temperature) / 4.0
        } else if body_temperature > 39.0 {
            (body_temperature - 39.0) / 3.0
        } else {
            0.0
        }
        .clamp(0.0, 1.0);

        needs.set_f64(
            "pain",
            (needs.get_f64("pain", 0.0) * (1.0 - dt * 0.01) + injury_progress.pain * dt)
                .clamp(0.0, 100.0),
        );
        needs.set_f64("bleeding", injury_progress.bleeding.clamp(0.0, 100.0));
        let infection_gain = injury_progress.infection_gain
            + environment.pathogen_exposure
                * (1.0 - needs.get_f64("hygiene", 100.0) / 100.0)
                * 0.18;
        needs.set_f64(
            "infection",
            (needs.get_f64("infection", 0.0) + infection_gain * dt).clamp(0.0, 100.0),
        );
        needs.set_f64(
            "hygiene",
            (needs.get_f64("hygiene", 100.0) - dt * (0.005 + activity * 0.025 + exposure * 0.04))
                .clamp(0.0, 100.0),
        );
        needs.set_f64(
            "oxygen",
            (needs.get_f64("oxygen", 100.0)
                + (environment.air_quality * 100.0 - needs.get_f64("oxygen", 100.0)) * dt * 0.08)
                .clamp(0.0, 100.0),
        );
        let stress_target = (report.thermal_stress * 55.0
            + needs.get_f64("pain", 0.0) * 0.45
            + needs.get_f64("infection", 0.0) * 0.3
            + (1.0 - environment.daylight) * 12.0)
            .clamp(0.0, 100.0);
        let stress = needs.get_f64("stress", 0.0)
            + (stress_target - needs.get_f64("stress", 0.0)) * dt * 0.08;
        needs.set_f64("stress", stress.clamp(0.0, 100.0));
        needs.set_f64(
            "morale",
            (needs.get_f64("morale", 100.0) - stress / 100.0 * dt * 0.06
                + environment.daylight * dt * 0.01)
                .clamp(0.0, 100.0),
        );
        needs.set_f64(
            "sickness",
            (needs.get_f64("infection", 0.0) * 0.7
                + (100.0 - needs.get_f64("oxygen", 100.0)) * 0.3)
                .clamp(0.0, 100.0),
        );

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
        danger += report.thermal_stress * 1.25;
        danger += (25.0 - needs.get_f64("oxygen", 100.0)).max(0.0) / 25.0 * 1.5;
        report.health_damage = critical_rate * danger * dt + injury_progress.health_damage;
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
        let equipment = Self::equipment_summary(entity);
        let body = entity.get_component("BodyCondition2D");
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
                },
                "equipment": equipment,
                "body": {
                    "blood_volume": body.map(|value| value.get_f64("blood_volume", 100.0)).unwrap_or(100.0),
                    "core_temperature_c": body.map(|value| value.get_f64("core_temperature_c", 36.8)).unwrap_or(36.8),
                    "immunity": body.map(|value| value.get_f64("immunity", 1.0)).unwrap_or(1.0),
                    "injuries": body.and_then(|value| value.get("injuries")).cloned().unwrap_or_else(|| json!([])),
                },
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

    /// Aggregates the equipped item records into gameplay-ready values.
    /// Item metadata may keep equipment fields at the root or below an
    /// `equipment` object, making imported item databases easy to adapt.
    pub fn equipment_summary(entity: &GameObject) -> EquipmentSummary {
        let Some(equipment) = entity.get_component("Equipment") else {
            return EquipmentSummary::default();
        };
        let mut summary = EquipmentSummary::default();
        if let Some(slot_bonuses) = equipment.get("stat_bonuses").and_then(Value::as_object) {
            for bonuses in slot_bonuses.values().filter_map(Value::as_object) {
                for (stat, value) in bonuses {
                    let bonus = finite_or(value.as_f64().unwrap_or(0.0), 0.0);
                    *summary.stat_bonuses.entry(stat.clone()).or_default() += bonus;
                }
            }
        }

        let records = equipment
            .get("equipped_items")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (primary_slot, record) in records {
            let item_id = record
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("item")
                .to_string();
            let metadata = record.get("metadata").unwrap_or(&Value::Null);
            let spec = equipment_metadata(metadata);
            let condition = equipment_condition(metadata);
            summary.equipped_count += 1;
            summary.item_ids.push(item_id);
            let occupied = record
                .get("occupied_slots")
                .and_then(Value::as_array)
                .map(|slots| {
                    slots
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                })
                .filter(|slots| !slots.is_empty())
                .unwrap_or_else(|| vec![primary_slot]);
            summary.occupied_slots.extend(occupied);
            summary.insulation += number_in(spec, "insulation", 0.0) * condition;
            summary.waterproofing += number_in(spec, "waterproofing", 0.0) * condition;
            summary.carry_capacity_bonus +=
                number_in(spec, "carry_capacity_bonus", 0.0) * condition;
            summary.noise += number_in(spec, "noise", 0.0).max(0.0) * condition;
            summary.total_weight += number_in(metadata, "weight", 0.0).max(0.0);
            summary.movement_multiplier *= number_in(spec, "movement_multiplier", 1.0)
                .clamp(0.05, 3.0)
                .powf(condition.max(0.05));
            if let Some(protection) = spec.get("protection") {
                if let Some(value) = protection.as_f64() {
                    *summary.protection.entry("all".to_string()).or_default() +=
                        finite_or(value, 0.0).max(0.0) * condition;
                } else if let Some(regions) = protection.as_object() {
                    for (region, value) in regions {
                        *summary.protection.entry(region.clone()).or_default() +=
                            finite_or(value.as_f64().unwrap_or(0.0), 0.0).max(0.0) * condition;
                    }
                }
            }
        }
        summary.item_ids.sort();
        summary.occupied_slots.sort();
        summary.occupied_slots.dedup();
        summary.insulation = summary.insulation.clamp(0.0, 1.0);
        summary.waterproofing = summary.waterproofing.clamp(0.0, 1.0);
        summary.noise = summary.noise.clamp(0.0, 100.0);
        summary.movement_multiplier = summary.movement_multiplier.clamp(0.05, 3.0);
        summary
    }

    pub fn effective_stat(entity: &GameObject, stat: &str) -> f64 {
        let base = entity
            .get_component("Stats")
            .map(|stats| stats.get_f64(stat, 0.0))
            .unwrap_or(0.0);
        base + Self::equipment_summary(entity)
            .stat_bonuses
            .get(stat)
            .copied()
            .unwrap_or(0.0)
    }

    /// Atomically removes one item from Inventory and equips it. If occupied
    /// gear cannot be returned to the inventory, no component is changed.
    pub fn equip_from_inventory(
        entity: &mut GameObject,
        item_id: &str,
        preferred_slot: Option<&str>,
    ) -> EquipmentChangeResult {
        ensure_component(entity, "Inventory");
        ensure_component(entity, "Equipment");
        let Some(metadata) = inventory_item_metadata(entity, item_id) else {
            return equipment_failure(item_id, preferred_slot.unwrap_or(""), "item_missing");
        };
        let spec = equipment_metadata(&metadata);
        let compatible_slots = string_list_in(spec, "compatible_slots");
        let slot = preferred_slot
            .filter(|value| !value.trim().is_empty())
            .map(ToString::to_string)
            .or_else(|| non_empty_string(spec, "slot"))
            .or_else(|| compatible_slots.first().cloned())
            .or_else(|| infer_equipment_slot(&metadata))
            .unwrap_or_else(|| "tool".to_string());
        if !compatible_slots.is_empty() && !compatible_slots.iter().any(|value| value == &slot) {
            return equipment_failure(item_id, &slot, "incompatible_slot");
        }
        if !equipment_requirements_met(entity, spec) {
            return equipment_failure(item_id, &slot, "requirements_not_met");
        }

        let mut inventory = entity
            .get_component("Inventory")
            .cloned()
            .expect("inventory was ensured");
        let mut equipment = entity
            .get_component("Equipment")
            .cloned()
            .expect("equipment was ensured");
        let locked = equipment.get_string_list("locked_slots");
        let mut occupied_slots = string_list_in(spec, "occupies_slots");
        if occupied_slots.is_empty() {
            occupied_slots.push(slot.clone());
        } else if !occupied_slots.iter().any(|value| value == &slot) {
            occupied_slots.insert(0, slot.clone());
        }
        occupied_slots.sort();
        occupied_slots.dedup();
        if occupied_slots
            .iter()
            .any(|occupied| locked.iter().any(|locked_slot| locked_slot == occupied))
        {
            return equipment_failure(item_id, &slot, "slot_locked");
        }
        if inventory.inventory_remove_item(item_id, 1) != 1 {
            return equipment_failure(item_id, &slot, "item_missing");
        }

        let mut slots = equipment
            .get("slots")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut records = equipment
            .get("equipped_items")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let displaced_keys = records
            .iter()
            .filter(|(_, record)| {
                record_slots(record)
                    .iter()
                    .any(|current| occupied_slots.iter().any(|next| next == current))
            })
            .map(|(key, _)| key.clone())
            .collect::<BTreeSet<_>>();
        let mut displaced_item_ids = Vec::new();
        let mut bonus_map = equipment
            .get("stat_bonuses")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for displaced_key in displaced_keys {
            let Some(record) = records.remove(&displaced_key) else {
                continue;
            };
            let displaced_id = record
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("item")
                .to_string();
            let displaced_metadata = record.get("metadata").cloned().unwrap_or_else(|| json!({}));
            if inventory.inventory_add_item(&displaced_id, 1, displaced_metadata) != 1 {
                return equipment_failure(item_id, &slot, "inventory_full_for_displaced_item");
            }
            for occupied in record_slots(&record) {
                slots.insert(occupied, Value::Null);
            }
            bonus_map.remove(&displaced_key);
            displaced_item_ids.push(displaced_id);
        }

        for occupied in &occupied_slots {
            slots.insert(occupied.clone(), json!(item_id));
        }
        let bonuses = spec.get("bonuses").cloned().unwrap_or_else(|| json!({}));
        bonus_map.insert(slot.clone(), bonuses);
        records.insert(
            slot.clone(),
            json!({
                "id": item_id,
                "metadata": metadata,
                "occupied_slots": occupied_slots,
                "equipped_at": 0.0,
                "broken": false,
            }),
        );
        equipment.set("slots", Value::Object(slots));
        equipment.set("equipped_items", Value::Object(records));
        equipment.set("stat_bonuses", Value::Object(bonus_map));
        if let Some(current) = entity.get_component_mut("Inventory") {
            *current = inventory;
        }
        if let Some(current) = entity.get_component_mut("Equipment") {
            *current = equipment;
        }
        EquipmentChangeResult {
            success: true,
            slot,
            item_id: item_id.to_string(),
            occupied_slots,
            displaced_item_ids,
            reason: "equipped".to_string(),
        }
    }

    pub fn unequip_to_inventory(entity: &mut GameObject, slot: &str) -> EquipmentChangeResult {
        ensure_component(entity, "Inventory");
        let Some(mut equipment) = entity.get_component("Equipment").cloned() else {
            return equipment_failure("", slot, "equipment_missing");
        };
        let mut records = equipment
            .get("equipped_items")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let record_key = records
            .iter()
            .find(|(key, record)| {
                *key == slot || record_slots(record).iter().any(|value| value == slot)
            })
            .map(|(key, _)| key.clone());
        let Some(record_key) = record_key else {
            return equipment_failure("", slot, "slot_empty");
        };
        let record = records
            .remove(&record_key)
            .expect("located equipment record must remain present");
        let item_id = record
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("item")
            .to_string();
        let metadata = record.get("metadata").cloned().unwrap_or_else(|| json!({}));
        let occupied_slots = record_slots(&record);
        let mut inventory = entity
            .get_component("Inventory")
            .cloned()
            .expect("inventory was ensured");
        if inventory.inventory_add_item(&item_id, 1, metadata) != 1 {
            return equipment_failure(&item_id, slot, "inventory_full");
        }
        let mut slots = equipment
            .get("slots")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for occupied in &occupied_slots {
            slots.insert(occupied.clone(), Value::Null);
        }
        let mut bonus_map = equipment
            .get("stat_bonuses")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        bonus_map.remove(&record_key);
        equipment.set("slots", Value::Object(slots));
        equipment.set("equipped_items", Value::Object(records));
        equipment.set("stat_bonuses", Value::Object(bonus_map));
        if let Some(current) = entity.get_component_mut("Inventory") {
            *current = inventory;
        }
        if let Some(current) = entity.get_component_mut("Equipment") {
            *current = equipment;
        }
        EquipmentChangeResult {
            success: true,
            slot: record_key,
            item_id,
            occupied_slots,
            displaced_item_ids: Vec::new(),
            reason: "unequipped".to_string(),
        }
    }

    pub fn degrade_equipped_item(entity: &mut GameObject, slot: &str, amount: f64) -> Option<f64> {
        let equipment = entity.get_component_mut("Equipment")?;
        let mut records = equipment.get("equipped_items")?.as_object()?.clone();
        let key = records
            .iter()
            .find(|(key, record)| {
                *key == slot || record_slots(record).iter().any(|value| value == slot)
            })
            .map(|(key, _)| key.clone())?;
        let record = records.get_mut(&key)?.as_object_mut()?;
        let metadata = record
            .entry("metadata".to_string())
            .or_insert_with(|| json!({}));
        let metadata = metadata.as_object_mut()?;
        let durability = metadata
            .entry("durability".to_string())
            .or_insert_with(|| json!({"current": 100.0, "max": 100.0}));
        let durability = durability.as_object_mut()?;
        let maximum = durability
            .get("max")
            .and_then(Value::as_f64)
            .unwrap_or(100.0)
            .max(0.0001);
        let current = durability
            .get("current")
            .and_then(Value::as_f64)
            .unwrap_or(maximum);
        let next = (current - finite_or(amount, 0.0).max(0.0)).clamp(0.0, maximum);
        durability.insert("current".to_string(), json!(next));
        record.insert("broken".to_string(), json!(next <= 0.0));
        equipment.set("equipped_items", Value::Object(records));
        Some(next / maximum)
    }

    pub fn apply_injury(
        entity: &mut GameObject,
        region: &str,
        injury_type: &str,
        severity: f64,
    ) -> InjuryResult {
        let severity = finite_or(severity, 0.0).clamp(0.0, 100.0);
        if severity <= 0.0 || region.trim().is_empty() || injury_type.trim().is_empty() {
            return InjuryResult {
                region: region.to_string(),
                injury_type: injury_type.to_string(),
                severity,
                reason: "invalid_injury".to_string(),
                ..InjuryResult::default()
            };
        }
        ensure_component(entity, "BodyCondition2D");
        let Some(body) = entity.get_component_mut("BodyCondition2D") else {
            return InjuryResult::default();
        };
        let injury_id = body
            .get("next_injury_id")
            .and_then(Value::as_u64)
            .unwrap_or(1);
        let mut injuries = body
            .get("injuries")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let kind = injury_type.trim().to_ascii_lowercase();
        let bleeding_rate = if matches!(kind.as_str(), "cut" | "bite" | "gunshot" | "laceration") {
            severity * 0.06
        } else {
            severity * 0.01
        };
        let infection_risk = if kind == "bite" {
            0.9
        } else if matches!(kind.as_str(), "cut" | "gunshot" | "laceration") {
            0.25
        } else {
            0.04
        };
        injuries.push(json!({
            "id": injury_id,
            "region": region,
            "type": kind,
            "severity": severity,
            "bleeding_rate": bleeding_rate,
            "infection_risk": infection_risk,
            "treated": false,
            "bandaged": false,
            "disinfected": false,
            "age_seconds": 0.0,
        }));
        body.set("injuries", Value::Array(injuries));
        body.set("next_injury_id", json!(injury_id.saturating_add(1)));
        InjuryResult {
            success: true,
            injury_id,
            region: region.to_string(),
            injury_type: kind,
            severity,
            reason: "injury_applied".to_string(),
        }
    }

    pub fn treat_injury_with_item(
        entity: &mut GameObject,
        injury_id: u64,
        item_id: &str,
    ) -> InjuryResult {
        let Some(metadata) = inventory_item_metadata(entity, item_id) else {
            return injury_failure(injury_id, "item_missing");
        };
        let treatment = metadata.get("treatment").unwrap_or(&metadata);
        let severity_reduction = number_in(treatment, "severity_reduction", 0.0).max(0.0);
        let bleeding_reduction = number_in(treatment, "bleeding_reduction", 0.0).max(0.0);
        let infection_reduction = number_in(treatment, "infection_reduction", 0.0).max(0.0);
        if severity_reduction <= 0.0 && bleeding_reduction <= 0.0 && infection_reduction <= 0.0 {
            return injury_failure(injury_id, "item_is_not_a_treatment");
        }
        let Some(mut body) = entity.get_component("BodyCondition2D").cloned() else {
            return injury_failure(injury_id, "body_condition_missing");
        };
        let mut injuries = body
            .get("injuries")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let Some(injury) = injuries
            .iter_mut()
            .find(|injury| injury.get("id").and_then(Value::as_u64) == Some(injury_id))
            .and_then(Value::as_object_mut)
        else {
            return injury_failure(injury_id, "injury_missing");
        };
        if entity
            .get_component_mut("Inventory")
            .map(|inventory| inventory.inventory_remove_item(item_id, 1))
            .unwrap_or(0)
            != 1
        {
            return injury_failure(injury_id, "item_missing");
        }
        let severity = (injury
            .get("severity")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            - severity_reduction)
            .max(0.0);
        let bleeding = (injury
            .get("bleeding_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            - bleeding_reduction)
            .max(0.0);
        let infection = (injury
            .get("infection_risk")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            - infection_reduction)
            .max(0.0);
        let region = injury
            .get("region")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let injury_type = injury
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        injury.insert("severity".to_string(), json!(severity));
        injury.insert("bleeding_rate".to_string(), json!(bleeding));
        injury.insert("infection_risk".to_string(), json!(infection));
        injury.insert("treated".to_string(), json!(true));
        injury.insert("bandaged".to_string(), json!(bleeding_reduction > 0.0));
        injury.insert("disinfected".to_string(), json!(infection_reduction > 0.0));
        if severity <= 0.0 && bleeding <= 0.0 {
            injuries.retain(|value| value.get("id").and_then(Value::as_u64) != Some(injury_id));
        }
        body.set("injuries", Value::Array(injuries));
        if let Some(current) = entity.get_component_mut("BodyCondition2D") {
            *current = body;
        }
        InjuryResult {
            success: true,
            injury_id,
            region,
            injury_type,
            severity,
            reason: "injury_treated".to_string(),
        }
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

#[derive(Debug, Clone, Copy, Default)]
struct InjuryProgress {
    active_injuries: usize,
    pain: f64,
    bleeding: f64,
    infection_gain: f64,
    health_damage: f64,
}

fn progress_body_condition(
    entity: &mut GameObject,
    dt: f64,
    environment: &SurvivalEnvironment2D,
) -> InjuryProgress {
    let Some(body) = entity.get_component_mut("BodyCondition2D") else {
        return InjuryProgress::default();
    };
    if !body.enabled || !body.get_bool("auto_progress", true) {
        return InjuryProgress::default();
    }
    let immunity = finite_or(body.get_f64("immunity", 1.0), 1.0).clamp(0.0, 4.0);
    let mut injuries = body
        .get("injuries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut progress = InjuryProgress::default();
    for injury in &mut injuries {
        let Some(injury) = injury.as_object_mut() else {
            continue;
        };
        let severity = finite_or(
            injury
                .get("severity")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            0.0,
        )
        .clamp(0.0, 100.0);
        if severity <= 0.0 {
            continue;
        }
        progress.active_injuries += 1;
        let treated = injury
            .get("treated")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let bandaged = injury
            .get("bandaged")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let disinfected = injury
            .get("disinfected")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let bleeding_rate = finite_or(
            injury
                .get("bleeding_rate")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            0.0,
        )
        .max(0.0)
            * if bandaged { 0.18 } else { 1.0 };
        let infection_risk = finite_or(
            injury
                .get("infection_risk")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            0.0,
        )
        .max(0.0)
            * if disinfected { 0.12 } else { 1.0 };
        progress.bleeding += bleeding_rate;
        progress.pain += severity * if treated { 0.012 } else { 0.025 };
        progress.infection_gain += infection_risk * severity / 100.0
            * (0.35 + environment.pathogen_exposure)
            / immunity.max(0.1);
        let age = injury
            .get("age_seconds")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            + dt;
        injury.insert("age_seconds".to_string(), json!(age));
        let healing = if treated { 0.035 } else { 0.003 } * immunity;
        injury.insert(
            "severity".to_string(),
            json!((severity - healing * dt).max(0.0)),
        );
    }
    injuries.retain(|injury| {
        injury
            .get("severity")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            > 0.001
    });
    let blood =
        (body.get_f64("blood_volume", 100.0) - progress.bleeding * dt * 0.08).clamp(0.0, 100.0);
    body.set_f64("blood_volume", blood);
    body.set("injuries", Value::Array(injuries));
    if blood < 45.0 {
        progress.health_damage += (45.0 - blood) / 45.0 * dt * 2.5;
    }
    progress.health_damage += progress.bleeding / 100.0 * dt;
    progress
}

fn equipment_metadata(metadata: &Value) -> &Value {
    metadata.get("equipment").unwrap_or(metadata)
}

fn equipment_condition(metadata: &Value) -> f64 {
    let Some(durability) = metadata.get("durability") else {
        return 1.0;
    };
    let Some(durability) = durability.as_object() else {
        return durability.as_f64().unwrap_or(1.0).clamp(0.0, 1.0);
    };
    let maximum = durability
        .get("max")
        .and_then(Value::as_f64)
        .unwrap_or(100.0)
        .max(0.0001);
    finite_or(
        durability
            .get("current")
            .and_then(Value::as_f64)
            .unwrap_or(maximum)
            / maximum,
        1.0,
    )
    .clamp(0.0, 1.0)
}

fn equipment_requirements_met(entity: &GameObject, spec: &Value) -> bool {
    let Some(requirements) = spec.get("requirements").and_then(Value::as_object) else {
        return true;
    };
    requirements.iter().all(|(stat, minimum)| {
        SurvivalSystems::effective_stat(entity, stat)
            >= finite_or(minimum.as_f64().unwrap_or(0.0), 0.0)
    })
}

fn record_slots(record: &Value) -> Vec<String> {
    record
        .get("occupied_slots")
        .and_then(Value::as_array)
        .map(|slots| {
            slots
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn string_list_in(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn non_empty_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn infer_equipment_slot(metadata: &Value) -> Option<String> {
    let category = metadata
        .get("category")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    match category.as_str() {
        "weapon" => Some("primary".to_string()),
        "armor" | "clothing" => Some("torso".to_string()),
        "helmet" | "headwear" => Some("head".to_string()),
        "backpack" | "container" => Some("back".to_string()),
        "tool" => Some("tool".to_string()),
        "trinket" | "accessory" => Some("trinket".to_string()),
        _ => None,
    }
}

fn number_in(value: &Value, key: &str, fallback: f64) -> f64 {
    finite_or(
        value.get(key).and_then(Value::as_f64).unwrap_or(fallback),
        fallback,
    )
}

fn equipment_failure(item_id: &str, slot: &str, reason: &str) -> EquipmentChangeResult {
    EquipmentChangeResult {
        item_id: item_id.to_string(),
        slot: slot.to_string(),
        reason: reason.to_string(),
        ..EquipmentChangeResult::default()
    }
}

fn injury_failure(injury_id: u64, reason: &str) -> InjuryResult {
    InjuryResult {
        injury_id,
        reason: reason.to_string(),
        ..InjuryResult::default()
    }
}

fn decay_need(component: &mut Component, field: &str, rate_field: &str, dt: f64) {
    let value = component.get_f64(field, 100.0) - component.get_f64(rate_field, 0.0).max(0.0) * dt;
    component.set_f64(field, value.clamp(0.0, 100.0));
}

fn decay_need_scaled(
    component: &mut Component,
    field: &str,
    rate_field: &str,
    dt: f64,
    scale: f64,
) {
    let value = component.get_f64(field, 100.0)
        - component.get_f64(rate_field, 0.0).max(0.0) * dt * finite_or(scale, 1.0).max(0.0);
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
    fn equipment_transactions_are_atomic_multi_slot_and_data_driven() {
        let mut actor = actor();
        actor.add_component(default_component("Stats").unwrap());
        actor
            .get_component_mut("Inventory")
            .unwrap()
            .inventory_add_item(
                "fire_axe",
                1,
                json!({
                    "weight": 2.4,
                    "category": "weapon",
                    "durability": {"current": 80.0, "max": 100.0},
                    "equipment": {
                        "slot": "primary",
                        "compatible_slots": ["primary"],
                        "occupies_slots": ["primary", "secondary"],
                        "requirements": {"strength": 4.0},
                        "bonuses": {"attack": 12.0, "strength": 2.0},
                        "protection": {"hands": 3.0},
                        "insulation": 0.2,
                        "noise": 8.0,
                        "movement_multiplier": 0.94
                    }
                }),
            );

        let equipped = SurvivalSystems::equip_from_inventory(&mut actor, "fire_axe", None);
        assert!(equipped.success, "{equipped:?}");
        assert_eq!(equipped.slot, "primary");
        assert_eq!(equipped.occupied_slots, vec!["primary", "secondary"]);
        assert_eq!(inventory_count(&actor, "fire_axe"), 0);
        let summary = SurvivalSystems::equipment_summary(&actor);
        assert_eq!(summary.equipped_count, 1);
        assert_eq!(summary.total_weight, 2.4);
        assert_eq!(summary.stat_bonuses["attack"], 12.0);
        assert_eq!(SurvivalSystems::effective_stat(&actor, "strength"), 7.0);
        assert!((summary.protection["hands"] - 2.4).abs() < 0.000_001);
        assert_eq!(
            SurvivalSystems::degrade_equipped_item(&mut actor, "secondary", 30.0),
            Some(0.5)
        );

        let unequipped = SurvivalSystems::unequip_to_inventory(&mut actor, "secondary");
        assert!(unequipped.success, "{unequipped:?}");
        assert_eq!(inventory_count(&actor, "fire_axe"), 1);
        assert_eq!(SurvivalSystems::equipment_summary(&actor).equipped_count, 0);
    }

    #[test]
    fn environment_injuries_and_treatment_form_one_reusable_survival_loop() {
        let mut actor = actor();
        actor.add_component(default_component("BodyCondition2D").unwrap());
        let injury = SurvivalSystems::apply_injury(&mut actor, "left_arm", "cut", 80.0);
        assert!(injury.success);
        let environment = SurvivalEnvironment2D {
            ambient_temperature_c: -12.0,
            wind_speed: 45.0,
            precipitation: 1.0,
            shelter: 0.0,
            pathogen_exposure: 0.5,
            daylight: 0.0,
            ..SurvivalEnvironment2D::default()
        };
        let report = SurvivalSystems::tick_entity_in_environment(&mut actor, 0.25, &environment);
        assert!(report.updated);
        assert_eq!(report.active_injuries, 1);
        assert!(report.health_damage > 0.0);
        assert!(report.core_temperature_c < 36.8);
        assert!(SurvivalSystems::need(&actor, "wetness").unwrap() > 0.0);
        assert!(SurvivalSystems::need(&actor, "bleeding").unwrap() > 0.0);

        actor
            .get_component_mut("Inventory")
            .unwrap()
            .inventory_add_item(
                "sterile_bandage",
                1,
                json!({
                    "treatment": {
                        "severity_reduction": 15.0,
                        "bleeding_reduction": 100.0,
                        "infection_reduction": 1.0
                    }
                }),
            );
        let treated = SurvivalSystems::treat_injury_with_item(
            &mut actor,
            injury.injury_id,
            "sterile_bandage",
        );
        assert!(treated.success, "{treated:?}");
        assert_eq!(inventory_count(&actor, "sterile_bandage"), 0);
        assert!(treated.severity < injury.severity);
        let state = SurvivalSystems::state(&actor);
        assert!(state["player"]["body"]["injuries"].is_array());
        assert!(state["player"]["equipment"]["movement_multiplier"].is_number());
    }

    #[test]
    fn survival_components_and_archetypes_are_available_without_project_code() {
        let registry = ComponentRegistry::new();
        for component in [
            "SurvivalNeeds",
            "SurvivalEnvironment2D",
            "BodyCondition2D",
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
        actor
            .get_component_mut("Inventory")
            .unwrap()
            .inventory_add_item(
                "field_jacket",
                1,
                json!({"equipment": {"slot": "torso", "bonuses": {"defense": 2.0}}}),
            );
        SurvivalSystems::set_need(&mut actor, "thirst", 10.0);
        let mut visual = default_component("VisualScript").unwrap();
        visual.set(
            "nodes",
            json!([
                {"id": "start", "type": "EventStart", "next": "use"},
                {"id": "use", "type": "UseInventoryItem", "item": "consumable", "next": "fatigue"},
                {"id": "fatigue", "type": "ModifySurvivalNeed", "need": "fatigue", "delta": 5.0, "next": "equip"},
                {"id": "equip", "type": "EquipInventoryItem", "item": "field_jacket", "slot": "torso", "next": "injury"},
                {"id": "injury", "type": "ApplyInjury", "region": "torso", "injury_type": "bruise", "severity": 12.0}
            ]),
        );
        actor.add_component(visual);
        let mut entities = vec![actor];

        VisualScriptRuntime::default().update_entities(&mut entities, 0.016, "PLAY");

        assert_eq!(SurvivalSystems::need(&entities[0], "thirst"), Some(35.0));
        assert_eq!(SurvivalSystems::need(&entities[0], "fatigue"), Some(5.0));
        assert_eq!(inventory_count(&entities[0], "consumable"), 0);
        assert_eq!(
            SurvivalSystems::equipment_summary(&entities[0]).item_ids,
            vec!["field_jacket"]
        );
        assert_eq!(
            entities[0]
                .get_component("BodyCondition2D")
                .unwrap()
                .get("injuries")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1)
        );
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
