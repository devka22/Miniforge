use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::engine::component::default_component;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct GameplaySystem {
    pub now: f64,
    pub stats: BTreeMap<String, usize>,
}

impl GameplaySystem {
    pub fn update_entities(&mut self, entities: &mut Vec<GameObject>, dt: f64, mode: &str) {
        if mode != "PLAY" {
            self.stats = BTreeMap::from([
                ("lifetimes".to_string(), 0),
                ("nav_agents".to_string(), 0),
                ("ai_agents".to_string(), 0),
                ("spawners".to_string(), 0),
                ("interactions".to_string(), 0),
                ("destroyed".to_string(), 0),
                ("damage_events".to_string(), 0),
            ]);
            return;
        }

        let dt = dt.clamp(0.0, 0.05);
        self.now += dt;
        let snapshot = entities.clone();
        let mut pending_destroy = Vec::new();
        let mut spawn_requests = Vec::new();

        for index in 0..entities.len() {
            if !entities[index].enabled {
                continue;
            }

            self.update_cooldown_timer_lifetime(&mut entities[index], dt, &mut pending_destroy);
            self.update_status_effects(&mut entities[index], dt);
            self.update_stat_regen(&mut entities[index], dt);
            self.update_state_machine(&mut entities[index], dt);
            self.update_tween(&mut entities[index], dt);
            self.update_nav_agent(&mut entities[index], dt);
            self.update_interaction(index, entities, &snapshot);

            if let Some(request) = self.update_spawner(&mut entities[index], dt) {
                spawn_requests.push(request);
            }
        }

        let mut damage_events = self.update_ai(entities, &mut pending_destroy);

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

        let before = entities.len();
        entities.retain(|entity| !pending_destroy.contains(&entity.id));
        let destroyed = before.saturating_sub(entities.len());
        damage_events += destroyed;

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
        ]);
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

    fn update_nav_agent(&self, entity: &mut GameObject, dt: f64) {
        let entity_x = entity.x;
        let entity_y = entity.y;
        let entity_speed = entity.speed;
        let path_empty = entity.path.is_empty();
        let Some(agent) = entity.get_component_mut("NavAgent") else {
            return;
        };
        if !agent.get_bool("has_destination", false) {
            return;
        }
        let destination_x = agent.get_f64("destination_x", entity_x);
        let destination_y = agent.get_f64("destination_y", entity_y);
        let stopping_distance = agent.get_f64("stopping_distance", 0.15);
        let speed = agent.get_f64("speed", entity_speed);
        let dx = destination_x - entity_x;
        let dy = destination_y - entity_y;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance <= stopping_distance {
            agent.nav_clear_destination();
            let _ = agent;
            entity.path.clear();
            entity.state = "IDLE".to_string();
            return;
        }

        let repath_timer = agent.get_f64("repath_timer", 0.0) + dt;
        let repath_interval = agent.get_f64("repath_interval", 0.25);
        let use_path =
            agent.get_bool("auto_repath", true) && agent.get_bool("avoid_obstacles", true);
        if use_path && (path_empty || repath_timer >= repath_interval) {
            agent.set_f64("repath_timer", 0.0);
            agent.set("last_path_length", json!(1));
            let _ = agent;
            entity.path = vec![(destination_x, destination_y)];
            entity.command = "NAVIGATE".to_string();
            entity.state = "MOVING".to_string();
        } else {
            agent.set_f64("repath_timer", repath_timer);
            let _ = agent;
            if distance > 0.0 {
                let step = (speed * dt).min(distance);
                entity.x += dx / distance * step;
                entity.y += dy / distance * step;
                entity.sync_to_components();
            }
        }
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

    fn update_ai(&mut self, entities: &mut [GameObject], pending_destroy: &mut Vec<u64>) -> usize {
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
                    attacker_mut.path = vec![(home_x + radius * 0.5, home_y)];
                    attacker_mut.command = "WANDER".to_string();
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
                    if attacker_mut.path.is_empty() {
                        attacker_mut.path = vec![(target.x, target.y)];
                        attacker_mut.command = "CHASE".to_string();
                    }
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
                let timer = (ai.get_f64("think_timer", 0.0) - 0.016).max(0.0);
                ai.set_f64("think_timer", timer);
            }
        }
        events
    }
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
