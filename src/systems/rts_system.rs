use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::engine::component::default_component;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct RTSSystem {
    pub now: f64,
    pub stats: BTreeMap<String, usize>,
}

impl RTSSystem {
    pub fn update_entities(&mut self, entities: &mut Vec<GameObject>, dt: f64, mode: &str) {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.1)
        } else {
            0.0
        };
        if mode != "PLAY" {
            self.stats = self.collect_stats(entities, 0, 0, 0, 0, TacticalCombatReport::default());
            return;
        }

        self.now += dt;
        let gathered = self.update_gathering(entities, dt);
        let completed_constructions = self.update_construction(entities, dt);
        let auto_queued = self.update_auto_queue(entities);
        let produced = self.update_production(entities, dt);
        let combat = self.update_tactical_combat(entities);
        self.update_fog_of_war(entities);
        self.stats = self.collect_stats(
            entities,
            gathered,
            produced,
            completed_constructions,
            auto_queued,
            combat,
        );
    }

    pub fn enqueue_production(
        producer: &mut GameObject,
        unit_type: &str,
        display_name: &str,
        build_time: f64,
        cost: Value,
    ) -> bool {
        if producer.get_component("ProductionQueue").is_none() {
            producer.add_component(default_component("ProductionQueue").expect("ProductionQueue"));
        }

        let Some(queue) = producer.get_component("ProductionQueue") else {
            return false;
        };
        let items = queue
            .get("queue")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if items.len() >= queue.get_usize("max_queue", 7) {
            if let Some(queue) = producer.get_component_mut("ProductionQueue") {
                queue.set("blocked_reason", json!("queue_full"));
            }
            return false;
        }

        if !Self::spend_cost(producer, &cost) {
            if let Some(queue) = producer.get_component_mut("ProductionQueue") {
                queue.set("blocked_reason", json!("missing_resources"));
            }
            return false;
        }

        let Some(queue) = producer.get_component_mut("ProductionQueue") else {
            return false;
        };
        let mut items = items;
        items.push(json!({
            "unit_type": unit_type,
            "display_name": display_name,
            "build_time": build_time.max(0.01),
            "elapsed": 0.0,
            "cost": cost,
        }));
        queue.set("queue", Value::Array(items));
        queue.set("blocked_reason", Value::Null);
        true
    }

    pub fn set_rally_point(producer: &mut GameObject, x: f64, y: f64) -> bool {
        let Some(queue) = producer.get_component_mut("ProductionQueue") else {
            return false;
        };
        queue.set_f64("rally_x", x);
        queue.set_f64("rally_y", y);
        true
    }

    fn update_auto_queue(&self, entities: &mut [GameObject]) -> usize {
        let mut queued = 0;
        for entity in entities {
            let Some(book) = entity.get_component("ProductionRecipeBook").cloned() else {
                continue;
            };
            if !book.get_bool("auto_queue", false) {
                continue;
            }
            let has_queue = entity
                .get_component("ProductionQueue")
                .and_then(|queue| queue.get("queue"))
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty());
            if has_queue {
                continue;
            }
            let preferred = book.get_string("preferred_recipe", "Worker");
            let recipes = book
                .get("recipes")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let recipe = recipes
                .iter()
                .find(|recipe| {
                    recipe.get("unit_type").and_then(Value::as_str) == Some(preferred.as_str())
                })
                .or_else(|| recipes.first());
            let Some(recipe) = recipe else {
                continue;
            };
            let unit_type = recipe
                .get("unit_type")
                .and_then(Value::as_str)
                .unwrap_or("Worker");
            let display_name = recipe
                .get("display_name")
                .and_then(Value::as_str)
                .unwrap_or(unit_type);
            let build_time = recipe
                .get("build_time")
                .and_then(Value::as_f64)
                .unwrap_or(3.0);
            let cost = recipe.get("cost").cloned().unwrap_or_else(|| json!({}));
            if Self::enqueue_production(entity, unit_type, display_name, build_time, cost) {
                queued += 1;
            }
        }
        queued
    }

    fn update_production(&self, entities: &mut Vec<GameObject>, dt: f64) -> usize {
        let mut requests = Vec::new();
        for entity in entities.iter_mut() {
            let owner_id = entity.id;
            let owner_team = Self::team_id(entity);
            let owner_position = (entity.x, entity.y);
            let Some(queue) = entity.get_component_mut("ProductionQueue") else {
                continue;
            };
            let mut items = queue
                .get("queue")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if items.is_empty() || !queue.get_bool("auto_start", true) {
                continue;
            }

            let speed = queue.get_f64("production_speed", 1.0).max(0.0);
            if let Some(first) = items.first_mut().and_then(Value::as_object_mut) {
                let elapsed =
                    first.get("elapsed").and_then(Value::as_f64).unwrap_or(0.0) + dt * speed;
                first.insert("elapsed".to_string(), json!(elapsed));
            }

            let complete = items
                .first()
                .map(|item| {
                    let elapsed = item.get("elapsed").and_then(Value::as_f64).unwrap_or(0.0);
                    let build_time = item
                        .get("build_time")
                        .and_then(Value::as_f64)
                        .unwrap_or(1.0)
                        .max(0.01);
                    elapsed >= build_time
                })
                .unwrap_or(false);

            if complete {
                let completed = items.remove(0);
                requests.push(ProductionSpawn {
                    owner_id,
                    team_id: owner_team,
                    unit_type: completed
                        .get("unit_type")
                        .and_then(Value::as_str)
                        .unwrap_or("Unit")
                        .to_string(),
                    display_name: completed
                        .get("display_name")
                        .and_then(Value::as_str)
                        .unwrap_or("Unit")
                        .to_string(),
                    x: queue.get_f64("rally_x", owner_position.0 + 2.0),
                    y: queue.get_f64("rally_y", owner_position.1),
                });
            }
            queue.set("queue", Value::Array(items));
        }

        let mut produced = 0;
        for request in requests {
            let mut unit = GameObject::new_unit(
                request.x + produced as f64 * 0.35,
                request.y,
                Some(request.display_name.clone()),
            );
            unit.tag = Self::tag_for_team(request.team_id).to_string();
            unit.layer = "Units".to_string();
            unit.add_component(default_component("Health").expect("Health"));
            unit.add_component(default_component("Stats").expect("Stats"));
            unit.add_component(default_component("NavAgent").expect("NavAgent"));
            unit.add_component(default_component("Commandable").expect("Commandable"));
            unit.add_component(default_component("Vision").expect("Vision"));
            Self::ensure_team_component(&mut unit, request.team_id);

            let unit_type = request.unit_type.to_lowercase();
            if unit_type.contains("worker") || unit_type.contains("harvester") {
                unit.add_component(default_component("Worker").expect("Worker"));
                unit.add_component(default_component("Inventory").expect("Inventory"));
                if let Some(commandable) = unit.get_component_mut("Commandable") {
                    commandable.set("can_gather", json!(true));
                    commandable.set("can_build", json!(true));
                }
            }
            if unit_type.contains("soldier")
                || unit_type.contains("marine")
                || unit_type.contains("fighter")
            {
                unit.add_component(default_component("DamageDealer").expect("DamageDealer"));
                unit.add_component(default_component("CombatTarget").expect("CombatTarget"));
                unit.add_component(default_component("ThreatSource").expect("ThreatSource"));
            }
            if let Some(blackboard) = default_component("Blackboard") {
                unit.add_component(blackboard);
            }
            unit.sync_to_components();
            if let Some(owner) = entities
                .iter_mut()
                .find(|entity| entity.id == request.owner_id)
                && let Some(queue) = owner.get_component_mut("ProductionQueue")
            {
                queue.set("last_produced_id", json!(unit.id));
            }
            entities.push(unit);
            produced += 1;
        }
        produced
    }

    fn update_tactical_combat(&mut self, entities: &mut Vec<GameObject>) -> TacticalCombatReport {
        let snapshot = entities.to_vec();
        let mut report = TacticalCombatReport::default();
        let mut target_updates = Vec::new();
        let mut move_orders = Vec::new();
        let mut damage_actions = Vec::new();

        for attacker in &snapshot {
            if !attacker.enabled || !Self::is_alive(attacker) {
                continue;
            }
            let Some(commandable) = attacker.get_component("Commandable") else {
                continue;
            };
            if !commandable.get_bool("can_attack", true) {
                continue;
            }
            let Some(damage) = attacker.get_component("DamageDealer") else {
                continue;
            };
            let team_id = Self::team_id(attacker);
            if team_id == 0 {
                continue;
            }
            let Some(target) = Self::resolve_combat_target(attacker, &snapshot, team_id) else {
                if let Some(target) = attacker.attack_move_target {
                    move_orders.push((attacker.id, target, "ATTACK_MOVE"));
                }
                continue;
            };

            report.targets_acquired += 1;
            target_updates.push((attacker.id, target.id));
            let range = attacker
                .get_component("CombatTarget")
                .map(|combat| combat.get_f64("attack_radius", 1.25))
                .unwrap_or(1.25)
                .max(damage.get_f64("range", 1.25));
            let target_distance = distance(attacker, target);
            if target_distance > range {
                move_orders.push((attacker.id, (target.x, target.y), "ENGAGE"));
                continue;
            }
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

        for (attacker_id, target_id) in target_updates {
            if let Some(attacker) = entities.iter_mut().find(|entity| entity.id == attacker_id)
                && let Some(combat) = attacker.get_component_mut("CombatTarget")
            {
                combat.set("target_id", json!(target_id));
            }
        }

        for (attacker_id, target, command) in move_orders {
            if let Some(attacker) = entities.iter_mut().find(|entity| entity.id == attacker_id) {
                let refresh_path = attacker
                    .path
                    .last()
                    .map(|last| {
                        ((last.0 - target.0).powi(2) + (last.1 - target.1).powi(2)).sqrt() > 0.75
                    })
                    .unwrap_or(true);
                if refresh_path {
                    attacker.path = vec![target];
                }
                attacker.command = command.to_string();
                attacker.state = "MOVING".to_string();
                if let Some(nav) = attacker.get_component_mut("NavAgent") {
                    nav.nav_set_destination(target.0, target.1);
                }
            }
        }

        let mut destroyed_ids = BTreeSet::new();
        for (attacker_id, target_id, amount) in damage_actions {
            if let Some(target) = entities.iter_mut().find(|entity| entity.id == target_id)
                && let Some(health) = target.get_component_mut("Health")
            {
                health.take_damage(amount);
                if !health.get_bool("alive", true) {
                    destroyed_ids.insert(target_id);
                }
                report.combat_events += 1;
            }
            if let Some(attacker) = entities.iter_mut().find(|entity| entity.id == attacker_id) {
                if let Some(damage) = attacker.get_component_mut("DamageDealer") {
                    damage.damage_mark_hit(target_id, self.now);
                }
                if destroyed_ids.contains(&target_id)
                    && let Some(stats) = attacker.get_component_mut("Stats")
                {
                    stats.stats_add_experience(15.0);
                }
            }
        }

        if !destroyed_ids.is_empty() {
            report.destroyed = destroyed_ids.len();
            entities.retain(|entity| !destroyed_ids.contains(&entity.id));
        }
        report
    }

    fn update_construction(&self, entities: &mut [GameObject], dt: f64) -> usize {
        let mut completions = Vec::new();
        for (index, entity) in entities.iter_mut().enumerate() {
            let Some(site) = entity.get_component_mut("ConstructionSite") else {
                continue;
            };
            if site.get_bool("completed", false) {
                continue;
            }
            let builders = site
                .get("builder_ids")
                .and_then(Value::as_array)
                .map(|items| items.len().max(1))
                .unwrap_or(1);
            let progress = site.get_f64("progress", 0.0)
                + dt * site.get_f64("build_rate", 1.0).max(0.0) * builders as f64;
            let build_time = site.get_f64("build_time", 8.0).max(0.01);
            site.set_f64("progress", progress.min(build_time));
            if progress >= build_time {
                site.set("completed", json!(true));
                completions.push(ConstructionCompletion {
                    index,
                    target_name: site.get_string("target_name", "Building"),
                    target_tag: site.get_string("target_tag", "Building"),
                    finished_components: site.get_string_list("finished_components"),
                });
            }
        }

        let mut completed = 0;
        for completion in completions {
            let Some(entity) = entities.get_mut(completion.index) else {
                continue;
            };
            entity.name = completion.target_name;
            entity.tag = completion.target_tag;
            entity.layer = "Buildings".to_string();
            entity.state = "READY".to_string();
            entity.command = "IDLE".to_string();
            for component_type in completion.finished_components {
                if entity.get_component(&component_type).is_none()
                    && let Some(component) = default_component(&component_type)
                {
                    entity.add_component(component);
                }
            }
            entity.sync_to_components();
            completed += 1;
        }
        completed
    }

    fn update_gathering(&self, entities: &mut [GameObject], dt: f64) -> usize {
        let snapshot = entities.to_vec();
        let mut gathered_events = 0;

        for worker in &snapshot {
            let Some(worker_component) = worker.get_component("Worker") else {
                continue;
            };
            let target_id = worker.gather_target_id.or_else(|| {
                worker_component
                    .get("gather_target_id")
                    .and_then(Value::as_u64)
            });
            let Some(target_id) = target_id else {
                continue;
            };
            let Some(resource) = snapshot.iter().find(|entity| entity.id == target_id) else {
                continue;
            };
            let Some(resource_component) = resource.get_component("ResourceNode") else {
                continue;
            };
            let range = worker_component
                .get_f64("gather_range", 1.35)
                .max(resource_component.get_f64("harvest_radius", 1.25));
            if distance(worker, resource) > range {
                continue;
            }

            let resource_type = resource_component.get_string("resource_type", "Gold");
            let amount = resource_component.get_f64("gather_rate", 10.0)
                * worker_component.get_f64("gather_efficiency", 1.0)
                * dt;
            let Some(resource_index) = find_index(entities, resource.id) else {
                continue;
            };
            let gathered = entities[resource_index]
                .get_component_mut("ResourceNode")
                .map(|node| node.gather(amount))
                .unwrap_or(0.0);
            if gathered <= 0.0 {
                continue;
            }

            let Some(worker_index) = find_index(entities, worker.id) else {
                continue;
            };
            let team_id = Self::team_id(&entities[worker_index]);
            let auto_deposit = entities[worker_index]
                .get_component("Worker")
                .map(|component| component.get_bool("auto_deposit", true))
                .unwrap_or(true);
            let carried = entities[worker_index]
                .get_component_mut("Worker")
                .map(|component| component.worker_add_resource(&resource_type, gathered))
                .unwrap_or(0.0);
            if carried <= 0.0 {
                continue;
            }

            if auto_deposit {
                let deposited =
                    Self::deposit_to_team_wallet(entities, team_id, &resource_type, carried);
                if deposited > 0.0
                    && let Some(worker) = entities
                        .get_mut(worker_index)
                        .and_then(|entity| entity.get_component_mut("Worker"))
                {
                    let remaining = (worker.get_f64("carrying_amount", 0.0) - deposited).max(0.0);
                    worker.set_f64("carrying_amount", remaining);
                    if remaining <= f64::EPSILON {
                        worker.set("carrying_type", Value::Null);
                    }
                }
            }
            gathered_events += 1;
        }
        gathered_events
    }

    fn update_fog_of_war(&self, entities: &mut [GameObject]) {
        let snapshot = entities.to_vec();
        for entity in entities {
            let Some(fog) = entity.get_component_mut("FogOfWar") else {
                continue;
            };
            let team_id = fog.get_i64("team_id", 1);
            let width = fog.get_i64("map_width", 60).max(1) as i32;
            let height = fog.get_i64("map_height", 40).max(1) as i32;
            let tile_size = fog.get_f64("tile_size", 1.0).max(0.0001);
            let mut visible = BTreeSet::new();

            for scout in &snapshot {
                if Self::team_id(scout) != team_id {
                    continue;
                }
                let Some(vision) = scout.get_component("Vision") else {
                    continue;
                };
                if !vision.get_bool("reveals_fog", true) {
                    continue;
                }
                let radius = vision.get_f64("radius", 7.0).max(0.0);
                let cx = (scout.x / tile_size).floor() as i32;
                let cy = (scout.y / tile_size).floor() as i32;
                let tile_radius = (radius / tile_size).ceil() as i32;
                for y in (cy - tile_radius).max(0)..=(cy + tile_radius).min(height - 1) {
                    for x in (cx - tile_radius).max(0)..=(cx + tile_radius).min(width - 1) {
                        let wx = (x as f64 + 0.5) * tile_size;
                        let wy = (y as f64 + 0.5) * tile_size;
                        if ((wx - scout.x).powi(2) + (wy - scout.y).powi(2)).sqrt() <= radius {
                            visible.insert((x, y));
                        }
                    }
                }
            }

            let mut explored = parse_tiles(fog.get("explored_tiles"));
            explored.extend(visible.iter().copied());
            fog.set("visible_tiles", tiles_to_value(&visible));
            fog.set("explored_tiles", tiles_to_value(&explored));
        }
    }

    fn collect_stats(
        &self,
        entities: &[GameObject],
        gathered: usize,
        produced: usize,
        completed_constructions: usize,
        auto_queued: usize,
        combat: TacticalCombatReport,
    ) -> BTreeMap<String, usize> {
        BTreeMap::from([
            (
                "controllers".to_string(),
                count_components(entities, "RTSController"),
            ),
            (
                "commandable".to_string(),
                count_components(entities, "Commandable"),
            ),
            ("workers".to_string(), count_components(entities, "Worker")),
            (
                "production_queues".to_string(),
                count_components(entities, "ProductionQueue"),
            ),
            (
                "construction_sites".to_string(),
                count_components(entities, "ConstructionSite"),
            ),
            (
                "fog_maps".to_string(),
                count_components(entities, "FogOfWar"),
            ),
            ("gather_events".to_string(), gathered),
            ("produced".to_string(), produced),
            (
                "completed_constructions".to_string(),
                completed_constructions,
            ),
            ("auto_queued".to_string(), auto_queued),
            ("targets_acquired".to_string(), combat.targets_acquired),
            ("combat_events".to_string(), combat.combat_events),
            ("destroyed".to_string(), combat.destroyed),
        ])
    }

    fn spend_cost(entity: &mut GameObject, cost: &Value) -> bool {
        let Some(costs) = cost.as_object() else {
            return true;
        };
        if costs.is_empty() {
            return true;
        }
        let Some(wallet) = entity.get_component_mut("EconomyWallet") else {
            return false;
        };
        let resources = wallet
            .get("resources")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        if !wallet.get_bool("allow_negative", false) {
            for (resource_type, amount) in costs {
                let amount = amount.as_f64().unwrap_or(0.0).max(0.0);
                let current = resources
                    .get(resource_type)
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                if current < amount {
                    return false;
                }
            }
        }
        for (resource_type, amount) in costs {
            wallet.economy_spend(resource_type, amount.as_f64().unwrap_or(0.0).max(0.0));
        }
        true
    }

    fn deposit_to_team_wallet(
        entities: &mut [GameObject],
        team_id: i64,
        resource_type: &str,
        amount: f64,
    ) -> f64 {
        for entity in entities {
            if Self::team_id(entity) != team_id {
                continue;
            }
            let Some(wallet) = entity.get_component_mut("EconomyWallet") else {
                continue;
            };
            let before = wallet
                .get("resources")
                .and_then(Value::as_object)
                .and_then(|resources| resources.get(resource_type))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let after = wallet.economy_add(resource_type, amount);
            return (after - before).max(0.0);
        }
        0.0
    }

    fn ensure_team_component(entity: &mut GameObject, team_id: i64) {
        if entity.get_component("Team").is_none() {
            entity.add_component(default_component("Team").expect("Team"));
        }
        if let Some(team) = entity.get_component_mut("Team") {
            team.set("team_id", json!(team_id));
            team.set(
                "team_name",
                json!(match team_id {
                    1 => "Player",
                    2 => "Enemy",
                    _ => "Neutral",
                }),
            );
            team.set(
                "color",
                json!(match team_id {
                    1 => [80, 160, 255],
                    2 => [255, 95, 95],
                    _ => [160, 160, 160],
                }),
            );
        }
    }

    fn team_id(entity: &GameObject) -> i64 {
        entity
            .get_component("Team")
            .map(|team| team.get_i64("team_id", 0))
            .unwrap_or_else(|| match entity.tag.as_str() {
                "Player" => 1,
                "Enemy" => 2,
                _ => 0,
            })
    }

    fn is_alive(entity: &GameObject) -> bool {
        entity
            .get_component("Health")
            .map(|health| health.get_bool("alive", true) && health.get_f64("health", 1.0) > 0.0)
            .unwrap_or(true)
    }

    fn resolve_combat_target<'a>(
        attacker: &GameObject,
        entities: &'a [GameObject],
        team_id: i64,
    ) -> Option<&'a GameObject> {
        let combat = attacker.get_component("CombatTarget");
        let lose_radius = combat
            .map(|component| component.get_f64("lose_radius", 10.0))
            .unwrap_or(10.0);
        if let Some(target_id) = combat
            .and_then(|component| component.get("target_id"))
            .and_then(Value::as_u64)
            && let Some(target) = entities.iter().find(|entity| entity.id == target_id)
            && Self::is_valid_hostile(attacker, target, team_id)
            && distance(attacker, target) <= lose_radius
        {
            return Some(target);
        }

        let aggro_radius = combat
            .map(|component| component.get_f64("aggro_radius", 6.0))
            .unwrap_or(6.0);
        Self::nearest_hostile(attacker, entities, team_id, aggro_radius)
    }

    fn nearest_hostile<'a>(
        attacker: &GameObject,
        entities: &'a [GameObject],
        team_id: i64,
        radius: f64,
    ) -> Option<&'a GameObject> {
        let mut best = None;
        let mut best_score = f64::INFINITY;
        for target in entities {
            if !Self::is_valid_hostile(attacker, target, team_id) {
                continue;
            }
            let current_distance = distance(attacker, target);
            if current_distance > radius {
                continue;
            }
            let priority = target
                .get_component("ThreatSource")
                .map(|threat| threat.get_f64("strength", 1.0))
                .unwrap_or(1.0);
            let score = current_distance - priority * 0.05;
            if score < best_score {
                best = Some(target);
                best_score = score;
            }
        }
        best
    }

    fn is_valid_hostile(attacker: &GameObject, target: &GameObject, attacker_team: i64) -> bool {
        if attacker.id == target.id || !target.enabled || !Self::is_alive(target) {
            return false;
        }
        let target_team = Self::team_id(target);
        if target_team != 0 {
            return target_team != attacker_team;
        }
        let target_tags = attacker
            .get_component("CombatTarget")
            .map(|combat| combat.get_string_list("target_tags"))
            .unwrap_or_else(|| vec!["Enemy".to_string()]);
        target_tags.iter().any(|tag| tag == &target.tag)
    }

    fn tag_for_team(team_id: i64) -> &'static str {
        match team_id {
            1 => "Player",
            2 => "Enemy",
            _ => "Neutral",
        }
    }
}

#[derive(Debug, Clone)]
struct ProductionSpawn {
    owner_id: u64,
    team_id: i64,
    unit_type: String,
    display_name: String,
    x: f64,
    y: f64,
}

#[derive(Debug, Clone)]
struct ConstructionCompletion {
    index: usize,
    target_name: String,
    target_tag: String,
    finished_components: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct TacticalCombatReport {
    targets_acquired: usize,
    combat_events: usize,
    destroyed: usize,
}

fn find_index(entities: &[GameObject], entity_id: u64) -> Option<usize> {
    entities.iter().position(|entity| entity.id == entity_id)
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

fn parse_tiles(value: Option<&Value>) -> BTreeSet<(i32, i32)> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let coords = item.as_array()?;
                    if coords.len() < 2 {
                        return None;
                    }
                    Some((coords[0].as_i64()? as i32, coords[1].as_i64()? as i32))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn tiles_to_value(tiles: &BTreeSet<(i32, i32)>) -> Value {
    Value::Array(tiles.iter().map(|(x, y)| json!([x, y])).collect())
}
