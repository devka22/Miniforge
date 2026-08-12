use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::engine::component::default_component;
use crate::engine::survival_systems::SurvivalSystems;
use crate::engine::survival_world::{NoiseEvent2D, SurvivalWorldSystems};
use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;
use crate::systems::command_system::CommandSystem;
use crate::systems::runtime_2d_system::respawn_entity;

#[derive(Debug, Clone, Default)]
pub struct GameplaySystem {
    pub now: f64,
    pub stats: BTreeMap<String, usize>,
}

impl GameplaySystem {
    pub fn update_entities(&mut self, entities: &mut Vec<GameObject>, dt: f64, mode: &str) {
        self.update_entities_internal(entities, None, dt, mode);
    }

    pub fn update_entities_with_grid(
        &mut self,
        entities: &mut Vec<GameObject>,
        grid: &Grid,
        dt: f64,
        mode: &str,
    ) {
        self.update_entities_internal(entities, Some(grid), dt, mode);
    }

    fn update_entities_internal(
        &mut self,
        entities: &mut Vec<GameObject>,
        grid: Option<&Grid>,
        dt: f64,
        mode: &str,
    ) {
        if mode != "PLAY" {
            self.stats = BTreeMap::from([
                ("lifetimes".to_string(), 0),
                ("nav_agents".to_string(), 0),
                ("ai_agents".to_string(), 0),
                ("spawners".to_string(), 0),
                ("interactions".to_string(), 0),
                ("destroyed".to_string(), 0),
                ("damage_events".to_string(), 0),
                ("respawned".to_string(), 0),
                ("loot_drops".to_string(), 0),
                ("survival_actors".to_string(), 0),
                ("survival_ui_bindings".to_string(), 0),
                ("harvest_respawns".to_string(), 0),
                ("sense_detections".to_string(), 0),
                ("world_interactions".to_string(), 0),
                ("nav_repaths".to_string(), 0),
                ("nav_blocked".to_string(), 0),
            ]);
            return;
        }

        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.05)
        } else {
            0.0
        };
        self.now += dt;
        let snapshot = entities.clone();
        let noise_events = snapshot
            .iter()
            .filter_map(|entity| {
                let emitter = entity.get_component("NoiseEmitter2D")?;
                let radius = emitter.get_f64("current_radius", 0.0);
                let intensity = emitter.get_f64("current_intensity", 0.0);
                (radius > 0.0 && intensity > 0.0).then(|| NoiseEvent2D {
                    source_id: entity.id,
                    x: entity.x,
                    y: entity.y,
                    radius,
                    intensity,
                    kind: emitter.get_string("last_kind", "movement"),
                    age: 0.0,
                    duration: 0.1,
                })
            })
            .collect::<Vec<_>>();
        let mut pending_destroy = Vec::new();
        let mut spawn_requests = Vec::new();
        let mut nav_repaths = 0usize;
        let mut nav_blocked = 0usize;
        let mut survival_actors = 0usize;
        let mut harvest_respawns = 0usize;
        let mut sense_detections = 0usize;
        let mut world_interactions = 0usize;

        for index in 0..entities.len() {
            if !entities[index].enabled {
                continue;
            }

            self.update_cooldown_timer_lifetime(&mut entities[index], dt, &mut pending_destroy);
            self.update_status_effects(&mut entities[index], dt);
            let survival = SurvivalSystems::tick_entity(&mut entities[index], dt);
            survival_actors += usize::from(survival.updated);
            harvest_respawns += usize::from(survival.resource_respawned);
            let perception = SurvivalWorldSystems::update_perception(
                &mut entities[index],
                &snapshot,
                &noise_events,
                dt,
            );
            sense_detections += usize::from(perception.detected);
            SurvivalWorldSystems::tick_noise(&mut entities[index], dt);
            world_interactions +=
                usize::from(SurvivalWorldSystems::tick_door(&mut entities[index], dt));
            self.update_stat_regen(&mut entities[index], dt);
            self.update_state_machine(&mut entities[index], dt);
            self.update_tween(&mut entities[index], dt);
            let nav = self.update_nav_agent(&mut entities[index], grid, dt);
            nav_repaths += usize::from(nav.repathed);
            nav_blocked += usize::from(nav.blocked);
            self.update_interaction(index, entities, &snapshot);

            if let Some(request) = self.update_spawner(&mut entities[index], dt) {
                spawn_requests.push(request);
            }
        }

        let mut damage_events = self.update_ai(entities, grid, &mut pending_destroy, dt);

        for request in spawn_requests {
            if entities
                .iter()
                .filter(|entity| request.spawned_ids.contains(&entity.id))
                .count()
                >= request.max_alive
            {
                continue;
            }
            let spawned = GameObject::new(
                request.x,
                request.y,
                Some(if request.prefab_name.is_empty() {
                    "Spawned".to_string()
                } else {
                    request.prefab_name
                }),
            );
            let spawned_id = spawned.id;
            entities.push(spawned);
            if let Some(spawner_entity) = entities
                .iter_mut()
                .find(|entity| entity.id == request.spawner_entity_id)
                && let Some(spawner) = spawner_entity.get_component_mut("Spawner")
            {
                let mut ids = spawner
                    .get("spawned_ids")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                ids.push(json!(spawned_id));
                spawner.set("spawned_ids", Value::Array(ids));
            }
        }

        let (respawned, loot_drops) = resolve_destruction(entities, &mut pending_destroy, self.now);
        let before = entities.len();
        let pending_destroy = pending_destroy.into_iter().collect::<BTreeSet<_>>();
        entities.retain(|entity| !pending_destroy.contains(&entity.id));
        let destroyed = before.saturating_sub(entities.len());
        damage_events += destroyed;
        let survival_ui_bindings = self.update_survival_ui_bindings(entities);

        self.stats = BTreeMap::from([
            (
                "lifetimes".to_string(),
                count_components(entities, "Lifetime"),
            ),
            (
                "nav_agents".to_string(),
                count_components(entities, "NavAgent"),
            ),
            (
                "ai_agents".to_string(),
                count_components(entities, "AIController"),
            ),
            (
                "spawners".to_string(),
                count_components(entities, "Spawner"),
            ),
            (
                "interactions".to_string(),
                count_components(entities, "Interaction"),
            ),
            ("destroyed".to_string(), destroyed),
            ("damage_events".to_string(), damage_events),
            ("respawned".to_string(), respawned),
            ("loot_drops".to_string(), loot_drops),
            ("survival_actors".to_string(), survival_actors),
            ("survival_ui_bindings".to_string(), survival_ui_bindings),
            ("harvest_respawns".to_string(), harvest_respawns),
            ("sense_detections".to_string(), sense_detections),
            ("world_interactions".to_string(), world_interactions),
            ("nav_repaths".to_string(), nav_repaths),
            ("nav_blocked".to_string(), nav_blocked),
        ]);
    }

    fn update_survival_ui_bindings(&self, entities: &mut [GameObject]) -> usize {
        let snapshot = entities.to_vec();
        let mut updated = 0;
        for entity in entities {
            let Some(binding) = entity.get_component("SurvivalUIBinding").cloned() else {
                continue;
            };
            if !binding.enabled {
                continue;
            }
            let target_tag = binding.get_string("target_tag", "Player");
            let Some(target) = snapshot
                .iter()
                .find(|candidate| candidate.tag == target_tag)
            else {
                continue;
            };
            let source = binding.get_string("source", "health");
            let Some((value, max)) = survival_binding_value(target, &source) else {
                continue;
            };
            let Some(ui) = entity.get_component_mut("UIElement") else {
                continue;
            };
            let output = binding.get_string("output", "progress");
            if output == "progress" {
                ui.set_f64("progress", value.clamp(0.0, max));
                ui.set_f64("max_progress", max.max(0.0001));
            }
            if output == "text" || binding.get_bool("show_value", true) {
                let precision = binding.get_i64("precision", 0).clamp(0, 6) as usize;
                let label = binding.get_string("label", "");
                let suffix = binding.get_string("suffix", "");
                let rendered = format!("{value:.precision$}{suffix}");
                ui.set(
                    "text",
                    json!(if label.is_empty() {
                        rendered
                    } else {
                        format!("{label} {rendered}")
                    }),
                );
            }
            updated += 1;
        }
        updated
    }

    fn update_cooldown_timer_lifetime(
        &self,
        entity: &mut GameObject,
        dt: f64,
        pending_destroy: &mut Vec<u64>,
    ) {
        if let Some(cooldown) = entity.get_component_mut("Cooldown") {
            cooldown.cooldown_tick(dt);
        }
        if let Some(timer) = entity.get_component_mut("Timer") {
            timer.timer_tick(dt);
        }
        if let Some(lifetime) = entity.get_component_mut("Lifetime") {
            let duration = lifetime.get_f64("duration", 5.0);
            if duration >= 0.0 {
                let elapsed = lifetime.get_f64("elapsed", 0.0) + dt;
                lifetime.set_f64("elapsed", elapsed);
                if lifetime.get_bool("destroy_on_expire", true) && elapsed >= duration {
                    pending_destroy.push(entity.id);
                }
            }
        }
    }

    fn update_status_effects(&self, entity: &mut GameObject, dt: f64) {
        let mut total_damage = 0.0;
        let mut total_heal = 0.0;
        if let Some(status) = entity.get_component_mut("StatusEffects") {
            let mut next_effects = Vec::new();
            let effects = status
                .get("effects")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for mut effect in effects {
                let elapsed = effect.get("elapsed").and_then(Value::as_f64).unwrap_or(0.0) + dt;
                let duration = effect
                    .get("duration")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let stacks = effect
                    .get("stacks")
                    .and_then(Value::as_i64)
                    .unwrap_or(1)
                    .max(1) as f64;
                if let Some(data) = effect.get("data").and_then(Value::as_object) {
                    total_damage += data
                        .get("damage_per_second")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0)
                        * dt
                        * stacks;
                    total_heal += data
                        .get("heal_per_second")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0)
                        * dt
                        * stacks;
                }
                if let Some(map) = effect.as_object_mut() {
                    map.insert("elapsed".to_string(), json!(elapsed));
                }
                if duration < 0.0 || elapsed < duration {
                    next_effects.push(effect);
                }
            }
            status.set("effects", Value::Array(next_effects));
        }
        if let Some(health) = entity.get_component_mut("Health") {
            if total_damage > 0.0 {
                health.take_damage(total_damage);
            }
            if total_heal > 0.0 {
                health.heal(total_heal);
            }
        }
    }

    fn update_stat_regen(&self, entity: &mut GameObject, dt: f64) {
        let regen = entity
            .get_component("Stats")
            .map(|stats| stats.get_f64("regen_per_second", 0.0))
            .unwrap_or(0.0);
        if regen > 0.0
            && let Some(health) = entity.get_component_mut("Health")
        {
            health.heal(regen * dt);
        }
    }

    fn update_state_machine(&self, entity: &mut GameObject, dt: f64) {
        let blackboard_values = entity
            .get_component("Blackboard")
            .and_then(|blackboard| blackboard.get("values"))
            .cloned()
            .unwrap_or_else(|| json!({}));
        let Some(machine) = entity.get_component_mut("StateMachine") else {
            return;
        };
        if machine.get_bool("auto_start", true)
            && machine.get_string("current_state", "").is_empty()
        {
            let initial = machine.get_string("initial_state", "Idle");
            machine.state_machine_set_state(&initial);
        }

        let time_in_state = machine.get_f64("time_in_state", 0.0) + dt;
        machine.set_f64("time_in_state", time_in_state);
        let current_state = machine.get_string("current_state", "Idle");
        let transitions = machine
            .get("transitions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        for transition in transitions {
            if let Some(from) = transition.get("from").and_then(Value::as_str)
                && from != current_state
            {
                continue;
            }
            if let Some(after) = transition.get("after").and_then(Value::as_f64)
                && time_in_state < after
            {
                continue;
            }
            if let Some(key) = transition.get("if").and_then(Value::as_str) {
                let expected = transition.get("equals").cloned().unwrap_or(json!(true));
                if blackboard_values.get(key).cloned().unwrap_or(Value::Null) != expected {
                    continue;
                }
            }
            let next = transition
                .get("to")
                .and_then(Value::as_str)
                .unwrap_or(&current_state)
                .to_string();
            machine.state_machine_set_state(&next);
            break;
        }
    }

    fn update_tween(&self, entity: &mut GameObject, dt: f64) {
        if let Some(tween) = entity.get_component_mut("Tween") {
            if !tween.get_bool("active", false) {
                return;
            }
            let elapsed = tween.get_f64("elapsed", 0.0) + dt;
            tween.set_f64("elapsed", elapsed);
            let path = tween.get_string("property_path", "x");
            let sample = tween.tween_sample();
            let duration = tween.get_f64("duration", 1.0);
            let looped = tween.get_bool("loop", false);
            let ping_pong = tween.get_bool("ping_pong", false);
            if elapsed >= duration {
                if looped {
                    tween.set_f64("elapsed", 0.0);
                    if ping_pong {
                        let from = tween.get_f64("from_value", 0.0);
                        let to = tween.get_f64("to_value", 1.0);
                        tween.set_f64("from_value", to);
                        tween.set_f64("to_value", from);
                    }
                } else {
                    tween.set("active", json!(false));
                }
            }
            let _ = tween;
            set_property_path(entity, &path, sample);
        }
    }

    fn update_nav_agent(
        &self,
        entity: &mut GameObject,
        grid: Option<&Grid>,
        dt: f64,
    ) -> NavUpdateOutcome {
        let mut outcome = NavUpdateOutcome::default();
        let entity_x = entity.x;
        let entity_y = entity.y;
        let entity_speed = entity.speed;
        let path_empty = entity.path.is_empty();
        let Some(agent) = entity.get_component_mut("NavAgent") else {
            return outcome;
        };
        if !agent.get_bool("has_destination", false) {
            return outcome;
        }
        let destination_x = agent.get_f64("destination_x", entity_x);
        let destination_y = agent.get_f64("destination_y", entity_y);
        let stopping_distance = agent.get_f64("stopping_distance", 0.15);
        let speed = agent.get_f64("speed", entity_speed).max(0.0);
        let dx = destination_x - entity_x;
        let dy = destination_y - entity_y;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance <= stopping_distance {
            agent.nav_clear_destination();
            agent.set("last_status", json!("arrived"));
            let _ = agent;
            entity.path.clear();
            entity.state = "IDLE".to_string();
            return outcome;
        }

        let repath_timer = agent.get_f64("repath_timer", 0.0) + dt;
        let repath_interval = agent.get_f64("repath_interval", 0.25).max(0.01);
        let use_path =
            agent.get_bool("auto_repath", true) && agent.get_bool("avoid_obstacles", true);
        if use_path && (path_empty || repath_timer >= repath_interval) {
            let path = grid
                .map(|grid| {
                    CommandSystem::build_path(
                        grid,
                        (entity_x.round() as i32, entity_y.round() as i32),
                        (destination_x.round() as i32, destination_y.round() as i32),
                    )
                })
                .unwrap_or_else(|| vec![(destination_x, destination_y)]);
            agent.set_f64("repath_timer", 0.0);
            agent.set("last_path_length", json!(path.len()));
            agent.set(
                "last_status",
                json!(if path.is_empty() { "blocked" } else { "moving" }),
            );
            let _ = agent;
            entity.speed = speed;
            if path.is_empty() {
                entity.path.clear();
                entity.command = "NAVIGATION_BLOCKED".to_string();
                entity.state = "BLOCKED".to_string();
                outcome.blocked = true;
            } else {
                entity.path = path;
                entity.command = "NAVIGATE".to_string();
                entity.state = "MOVING".to_string();
                outcome.repathed = true;
            }
            return outcome;
        }

        if use_path {
            agent.set_f64("repath_timer", repath_timer);
            let _ = agent;
            entity.speed = speed;
            // MovementSystem is the sole owner of path traversal. Keeping the
            // NavAgent stationary here prevents doubled speed and obstacle
            // tunnelling between repath ticks.
            return outcome;
        }

        agent.set_f64("repath_timer", repath_timer);
        agent.set("last_status", json!("direct"));
        let _ = agent;
        entity.speed = speed;
        if distance > 0.0 {
            let step = (speed * dt).min(distance);
            entity.x += dx / distance * step;
            entity.y += dy / distance * step;
            entity.sync_to_components();
        }
        outcome
    }

    fn update_interaction(
        &self,
        index: usize,
        entities: &mut [GameObject],
        snapshot: &[GameObject],
    ) {
        let Some(interaction) = entities[index].get_component("Interaction").cloned() else {
            return;
        };
        if interaction.get_bool("single_use", false) && interaction.get_bool("used", false) {
            return;
        }
        let required_tag = interaction.get_string("requires_tag", "Player");
        let radius = interaction.get_f64("radius", 1.25);
        let origin = &snapshot[index];
        let active = snapshot.iter().any(|candidate| {
            candidate.id != origin.id
                && candidate.tag == required_tag
                && distance(origin, candidate) <= radius
        });
        if let Some(interaction) = entities[index].get_component_mut("Interaction") {
            interaction.set("active", json!(active));
        }
    }

    fn update_spawner(&self, entity: &mut GameObject, dt: f64) -> Option<SpawnRequest> {
        let spawner_entity_id = entity.id;
        let entity_x = entity.x;
        let entity_y = entity.y;
        let spawner = entity.get_component_mut("Spawner")?;

        let spawned_ids: Vec<u64> = spawner
            .get("spawned_ids")
            .and_then(Value::as_array)
            .map(|ids| ids.iter().filter_map(Value::as_u64).collect())
            .unwrap_or_default();
        let max_alive = spawner.get_i64("max_alive", 3).max(0) as usize;
        if spawned_ids.len() >= max_alive {
            return None;
        }

        let mut should_spawn = false;
        if spawner.get_bool("spawn_on_start", false) && !spawner.get_bool("started", false) {
            should_spawn = true;
            spawner.set("started", json!(true));
        }

        let elapsed = spawner.get_f64("elapsed", 0.0) + dt;
        if elapsed >= spawner.get_f64("spawn_interval", 5.0) {
            should_spawn = true;
            spawner.set_f64("elapsed", 0.0);
        } else {
            spawner.set_f64("elapsed", elapsed);
        }

        if !should_spawn {
            return None;
        }

        let radius = spawner.get_f64("spawn_radius", 2.0).max(0.0);
        let seed = (self.now * 997.0 + entity_x * 31.0 + entity_y * 17.0).sin();
        let angle = seed.abs() * std::f64::consts::TAU;
        let distance = radius * (0.35 + 0.65 * seed.cos().abs());
        Some(SpawnRequest {
            spawner_entity_id,
            prefab_name: spawner.get_string("prefab_name", ""),
            x: entity_x + angle.cos() * distance,
            y: entity_y + angle.sin() * distance,
            max_alive,
            spawned_ids,
        })
    }

    fn update_ai(
        &mut self,
        entities: &mut [GameObject],
        grid: Option<&Grid>,
        pending_destroy: &mut Vec<u64>,
        dt: f64,
    ) -> usize {
        let snapshot = entities.to_vec();
        let mut damage_actions = Vec::new();

        for attacker in &snapshot {
            let Some(ai) = attacker.get_component("AIController") else {
                continue;
            };
            let behavior = ai.get_string("behavior", "idle");
            if behavior == "idle" {
                continue;
            }
            if ai.get_f64("think_timer", 0.0) > 0.0 {
                continue;
            }
            let target_tags = ai.get_string_list("target_tags");
            let detection_radius = ai.get_f64("detection_radius", 6.0);
            let attack_radius = ai.get_f64("attack_radius", 1.25);
            let target = find_nearest_target(attacker, &snapshot, &target_tags, detection_radius);

            let Some(target) = target else {
                if behavior == "wander"
                    && let Some(attacker_mut) =
                        entities.iter_mut().find(|entity| entity.id == attacker.id)
                    && attacker_mut.path.is_empty()
                {
                    let home_x = ai.get_f64("home_x", attacker.x);
                    let home_y = ai.get_f64("home_y", attacker.y);
                    let radius = ai.get_f64("wander_radius", 5.0);
                    set_ai_destination(
                        attacker_mut,
                        (home_x + radius * 0.5, home_y),
                        "WANDER",
                        grid,
                    );
                }
                continue;
            };

            let target_distance = distance(attacker, target);
            if matches!(behavior.as_str(), "chase" | "attack" | "guard")
                && target_distance > attack_radius
            {
                if let Some(attacker_mut) =
                    entities.iter_mut().find(|entity| entity.id == attacker.id)
                {
                    set_ai_destination(attacker_mut, (target.x, target.y), "CHASE", grid);
                    if let Some(ai_mut) = attacker_mut.get_component_mut("AIController") {
                        ai_mut.set("state", json!("chase"));
                        ai_mut.set("target_id", json!(target.id));
                        ai_mut.set_f64("think_timer", ai.get_f64("think_interval", 0.25).max(0.02));
                    }
                }
                continue;
            }

            if matches!(behavior.as_str(), "attack" | "guard") {
                let Some(damage) = attacker.get_component("DamageDealer") else {
                    continue;
                };
                if damage.damage_can_hit(target.id, self.now) {
                    let mut amount = damage.get_f64("damage", 10.0);
                    if let Some(stats) = attacker.get_component("Stats") {
                        amount += stats.stats_effective_attack() * 0.25;
                    }
                    if let Some(target_stats) = target.get_component("Stats") {
                        amount = (amount - target_stats.stats_effective_defense() * 0.1).max(0.0);
                    }
                    damage_actions.push((attacker.id, target.id, amount));
                }
            }
        }

        let mut events = 0;
        for (attacker_id, target_id, amount) in damage_actions {
            if let Some(target) = entities.iter_mut().find(|entity| entity.id == target_id)
                && let Some(health) = target.get_component_mut("Health")
            {
                health.take_damage(amount);
                if !health.get_bool("alive", true) {
                    pending_destroy.push(target_id);
                }
                events += 1;
            }
            if let Some(attacker) = entities.iter_mut().find(|entity| entity.id == attacker_id) {
                if let Some(damage) = attacker.get_component_mut("DamageDealer") {
                    damage.damage_mark_hit(target_id, self.now);
                }
                if let Some(ai) = attacker.get_component_mut("AIController") {
                    ai.set("state", json!("attack"));
                    ai.set_f64("think_timer", ai.get_f64("think_interval", 0.25).max(0.02));
                }
            }
        }
        for entity in entities {
            if let Some(ai) = entity.get_component_mut("AIController") {
                let timer = (ai.get_f64("think_timer", 0.0) - dt).max(0.0);
                ai.set_f64("think_timer", timer);
            }
        }
        events
    }
}

fn survival_binding_value(entity: &GameObject, source: &str) -> Option<(f64, f64)> {
    let source = source
        .trim()
        .strip_prefix("player.")
        .unwrap_or(source.trim());
    match source {
        "health" | "health.value" => {
            let health = entity.get_component("Health")?;
            Some((
                health.get_f64("health", 0.0),
                health.get_f64("max_health", 100.0).max(0.0001),
            ))
        }
        "health.percent" | "health_percent" => {
            let health = entity.get_component("Health")?;
            let max = health.get_f64("max_health", 100.0).max(0.0001);
            Some((health.get_f64("health", 0.0) / max, 1.0))
        }
        "inventory.weight" | "inventory_weight" => {
            let inventory = entity.get_component("Inventory")?;
            let value = inventory.inventory_weight();
            let configured_max = inventory.get_f64("max_weight", 0.0);
            Some((value, configured_max.max(value).max(0.0001)))
        }
        "inventory.slots_used" | "inventory_slots_used" => {
            let inventory = entity.get_component("Inventory")?;
            let value = inventory
                .get("items")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0) as f64;
            Some((value, inventory.get_i64("capacity", 0).max(1) as f64))
        }
        source => {
            let need = source.strip_prefix("needs.").unwrap_or(source);
            SurvivalSystems::need(entity, need).map(|value| (value, 100.0))
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct NavUpdateOutcome {
    repathed: bool,
    blocked: bool,
}

#[derive(Debug, Clone)]
struct SpawnRequest {
    spawner_entity_id: u64,
    prefab_name: String,
    x: f64,
    y: f64,
    max_alive: usize,
    spawned_ids: Vec<u64>,
}

fn count_components(entities: &[GameObject], component_type: &str) -> usize {
    entities
        .iter()
        .filter(|entity| entity.get_component(component_type).is_some())
        .count()
}

fn distance(first: &GameObject, second: &GameObject) -> f64 {
    ((first.x - second.x).powi(2) + (first.y - second.y).powi(2)).sqrt()
}

fn find_nearest_target<'a>(
    origin: &GameObject,
    entities: &'a [GameObject],
    target_tags: &[String],
    radius: f64,
) -> Option<&'a GameObject> {
    let mut best = None;
    let mut best_distance = radius;
    for entity in entities {
        if entity.id == origin.id || !entity.enabled {
            continue;
        }
        let tagged = target_tags.iter().any(|tag| tag == &entity.tag);
        let enemy_team = match (origin.get_component("Team"), entity.get_component("Team")) {
            (Some(origin_team), Some(target_team)) => {
                origin_team.get_i64("team_id", 0) != target_team.get_i64("team_id", 0)
            }
            _ => false,
        };
        if !tagged && !enemy_team {
            continue;
        }
        let current_distance = distance(origin, entity);
        if current_distance <= best_distance {
            best = Some(entity);
            best_distance = current_distance;
        }
    }
    best
}

fn set_ai_destination(
    entity: &mut GameObject,
    target: (f64, f64),
    command: &str,
    grid: Option<&Grid>,
) -> bool {
    if let Some(agent) = entity.get_component_mut("NavAgent") {
        agent.nav_set_destination(target.0, target.1);
        agent.set("last_status", json!("queued"));
        let _ = agent;
        entity.command = command.to_string();
        entity.state = "MOVING".to_string();
        return true;
    }

    let path = grid
        .map(|grid| {
            CommandSystem::build_path(
                grid,
                (entity.x.round() as i32, entity.y.round() as i32),
                (target.0.round() as i32, target.1.round() as i32),
            )
        })
        .unwrap_or_else(|| vec![target]);
    if path.is_empty() {
        entity.path.clear();
        entity.command = "NAVIGATION_BLOCKED".to_string();
        entity.state = "BLOCKED".to_string();
        return false;
    }
    entity.path = path;
    entity.command = command.to_string();
    entity.state = "MOVING".to_string();
    true
}

fn set_property_path(entity: &mut GameObject, path: &str, value: f64) -> bool {
    if let Some((component_type, attr)) = path.split_once('.')
        && let Some(component) = entity.get_component_mut(component_type)
    {
        component.set_f64(attr, value);
        return true;
    }
    match path {
        "x" => entity.x = value,
        "y" => entity.y = value,
        "rotation" => entity.rotation = value,
        "scale_x" => entity.scale_x = value,
        "scale_y" => entity.scale_y = value,
        "width" => entity.width = value,
        "height" => entity.height = value,
        _ => return false,
    }
    entity.sync_to_components();
    true
}

fn resolve_destruction(
    entities: &mut Vec<GameObject>,
    pending_destroy: &mut Vec<u64>,
    now: f64,
) -> (usize, usize) {
    let mut respawned = 0;
    let mut retained_destroy = Vec::new();
    let mut seen = BTreeSet::new();
    let mut loot_spawns = Vec::new();

    for entity_id in pending_destroy.drain(..) {
        if !seen.insert(entity_id) {
            continue;
        }
        let Some(index) = entities.iter().position(|entity| entity.id == entity_id) else {
            continue;
        };
        if respawn_entity(&mut entities[index]) {
            respawned += 1;
            continue;
        }
        loot_spawns.extend(loot_spawns_for(&entities[index], now));
        retained_destroy.push(entity_id);
    }

    let loot_drops = loot_spawns.len();
    entities.extend(loot_spawns);
    *pending_destroy = retained_destroy;
    (respawned, loot_drops)
}

fn loot_spawns_for(source: &GameObject, now: f64) -> Vec<GameObject> {
    let Some(loot) = source.get_component("LootTable") else {
        return Vec::new();
    };
    if !loot.enabled {
        return Vec::new();
    }
    let entries = loot
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if entries.is_empty() {
        return Vec::new();
    }

    let rolls = loot.get_i64("rolls", 1).max(0) as usize;
    let radius = loot.get_f64("drop_radius", 0.5).max(0.0);
    let mut drops = Vec::new();
    for roll in 0..rolls {
        let Some(entry) = choose_loot_entry(&entries, source.id, roll, now) else {
            continue;
        };
        let item_id = entry
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("loot")
            .to_string();
        let min = entry.get("min").and_then(Value::as_i64).unwrap_or(1);
        let max = entry
            .get("max")
            .and_then(Value::as_i64)
            .unwrap_or(min)
            .max(min);
        let quantity = min + ((source.id as i64 + roll as i64) % (max - min + 1));
        let angle = ((source.id as f64 + now * 97.0 + roll as f64 * 13.0)
            .sin()
            .abs())
            * std::f64::consts::TAU;
        let distance = radius * (0.25 + 0.75 * (roll as f64 + 1.0) / rolls.max(1) as f64);

        let mut drop = GameObject::new(
            source.x + angle.cos() * distance,
            source.y + angle.sin() * distance,
            Some(format!("Loot_{item_id}")),
        );
        drop.tag = "Neutral".to_string();
        drop.layer = "Pickups".to_string();
        drop.radius = 0.25;
        drop.width = 0.5;
        drop.height = 0.5;
        if let Some(mut interaction) = default_component("Interaction") {
            interaction.set("prompt", json!(format!("Pick up {item_id}")));
            interaction.set("action_name", json!("pickup"));
            interaction.set("single_use", json!(true));
            drop.add_component(interaction);
        }
        if let Some(mut blackboard) = default_component("Blackboard") {
            blackboard.blackboard_set("item_id", json!(item_id));
            blackboard.blackboard_set("quantity", json!(quantity));
            drop.add_component(blackboard);
        }
        if let Some(mut lifetime) = default_component("Lifetime") {
            lifetime.set_f64("duration", loot.get_f64("lifetime", 30.0));
            drop.add_component(lifetime);
        }
        drop.sync_to_components();
        drops.push(drop);
    }
    drops
}

fn choose_loot_entry(entries: &[Value], source_id: u64, roll: usize, now: f64) -> Option<Value> {
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
        return entries.first().cloned();
    }
    let mut cursor = ((source_id as f64 * 0.618_033_988_75 + roll as f64 + now).fract() * total)
        .clamp(0.0, total);
    for entry in entries {
        cursor -= entry
            .get("weight")
            .and_then(Value::as_f64)
            .unwrap_or(1.0)
            .max(0.0);
        if cursor <= 0.0 {
            return Some(entry.clone());
        }
    }
    entries.last().cloned()
}

pub fn add_default_component(entity: &mut GameObject, component_type: &str) -> bool {
    if entity.get_component(component_type).is_some() {
        return true;
    }
    let Some(component) = default_component(component_type) else {
        return false;
    };
    entity.add_component(component);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nav_actor(x: f64, y: f64, destination: (f64, f64)) -> GameObject {
        let mut actor = GameObject::new(x, y, Some("NavActor".to_string()));
        assert!(add_default_component(&mut actor, "NavAgent"));
        actor
            .get_component_mut("NavAgent")
            .unwrap()
            .nav_set_destination(destination.0, destination.1);
        actor
    }

    #[test]
    fn nav_agent_builds_grid_path_around_obstacles() {
        let mut grid = Grid::new(5, 3, 1, 1);
        grid.set_tile(2, 1, 1);
        let mut entities = vec![nav_actor(0.0, 1.0, (4.0, 1.0))];
        let mut system = GameplaySystem::default();

        system.update_entities_with_grid(&mut entities, &grid, 0.016, "PLAY");

        assert!(!entities[0].path.is_empty());
        assert!(
            !entities[0]
                .path
                .iter()
                .any(|point| point.0.round() == 2.0 && point.1.round() == 1.0)
        );
        assert_eq!(system.stats.get("nav_repaths"), Some(&1));
        assert_eq!(system.stats.get("nav_blocked"), Some(&0));
    }

    #[test]
    fn nav_agent_reports_blocked_instead_of_crossing_solid_wall() {
        let mut grid = Grid::new(5, 3, 1, 1);
        for y in 0..3 {
            grid.set_tile(2, y, 1);
        }
        let mut entities = vec![nav_actor(0.0, 1.0, (4.0, 1.0))];
        let mut system = GameplaySystem::default();

        system.update_entities_with_grid(&mut entities, &grid, 0.016, "PLAY");

        assert!(entities[0].path.is_empty());
        assert_eq!(entities[0].command, "NAVIGATION_BLOCKED");
        assert_eq!(entities[0].state, "BLOCKED");
        assert_eq!(system.stats.get("nav_blocked"), Some(&1));
    }

    #[test]
    fn path_following_is_left_to_movement_system_between_repaths() {
        let grid = Grid::new(6, 2, 1, 1);
        let mut entities = vec![nav_actor(0.0, 0.0, (5.0, 0.0))];
        let mut system = GameplaySystem::default();
        system.update_entities_with_grid(&mut entities, &grid, 0.016, "PLAY");
        let position = (entities[0].x, entities[0].y);
        assert!(!entities[0].path.is_empty());

        system.update_entities_with_grid(&mut entities, &grid, 0.016, "PLAY");

        assert_eq!((entities[0].x, entities[0].y), position);
        assert_eq!(system.stats.get("nav_repaths"), Some(&0));
    }

    #[test]
    fn gameplay_system_updates_survival_components_without_scripts() {
        let mut actor = GameObject::new_unit(0.0, 0.0, Some("Actor".to_string()));
        actor.add_component(default_component("Health").unwrap());
        let mut needs = default_component("SurvivalNeeds").unwrap();
        needs.set_f64("hunger_decay_per_second", 10.0);
        actor.add_component(needs);
        let mut entities = vec![actor];
        let mut system = GameplaySystem::default();

        system.update_entities(&mut entities, 0.05, "PLAY");

        assert!(
            entities[0]
                .get_component("SurvivalNeeds")
                .unwrap()
                .get_f64("hunger", 100.0)
                < 100.0
        );
        assert_eq!(system.stats.get("survival_actors"), Some(&1));
    }

    #[test]
    fn gameplay_system_advances_senses_noise_and_doors_without_scripts() {
        let mut hunter = GameObject::new(0.0, 0.0, Some("Hunter".to_string()));
        hunter.tag = "Enemy".to_string();
        hunter.add_component(default_component("Senses2D").unwrap());

        let mut player = GameObject::new(-12.0, 0.0, Some("Player".to_string()));
        player.tag = "Player".to_string();
        player.add_component(default_component("NoiseEmitter2D").unwrap());
        let _ = SurvivalWorldSystems::emit_noise(&mut player, "combat", 1.4);

        let mut door = GameObject::new(2.0, 2.0, Some("Door".to_string()));
        door.add_component(default_component("Door2D").unwrap());
        let _ = SurvivalWorldSystems::door_action(
            &mut door,
            crate::engine::survival_world::DoorCommand::Open,
        );

        let mut entities = vec![hunter, player, door];
        let mut system = GameplaySystem::default();
        system.update_entities(&mut entities, 0.05, "PLAY");

        assert!(
            entities[0]
                .get_component("Senses2D")
                .unwrap()
                .get_f64("alertness", 0.0)
                > 0.0
        );
        assert!(
            entities[2]
                .get_component("Door2D")
                .unwrap()
                .get_f64("open_progress", 0.0)
                > 0.0
        );
        assert_eq!(system.stats.get("sense_detections"), Some(&1));
        assert_eq!(system.stats.get("world_interactions"), Some(&1));
    }

    #[test]
    fn survival_ui_binding_updates_progress_without_scripts() {
        let mut actor = GameObject::new_unit(0.0, 0.0, Some("Actor".to_string()));
        actor.tag = "Player".to_string();
        actor.add_component(default_component("Health").unwrap());
        actor
            .get_component_mut("Health")
            .unwrap()
            .set_f64("health", 42.0);

        let mut hud = GameObject::new(0.0, 0.0, Some("SurvivalHealth".to_string()));
        hud.add_component(default_component("UIElement").unwrap());
        hud.add_component(default_component("SurvivalUIBinding").unwrap());
        let mut entities = vec![actor, hud];
        let mut system = GameplaySystem::default();

        system.update_entities(&mut entities, 0.0, "PLAY");

        let ui = entities[1].get_component("UIElement").unwrap();
        assert_eq!(ui.get_f64("progress", 0.0), 42.0);
        assert_eq!(ui.get_f64("max_progress", 0.0), 100.0);
        assert_eq!(ui.get_string("text", ""), "Health 42");
        assert_eq!(system.stats.get("survival_ui_bindings"), Some(&1));
    }
}
