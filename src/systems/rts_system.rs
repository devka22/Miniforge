use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::engine::component::default_component;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct RTSSystem {
    pub stats: BTreeMap<String, usize>,
}

impl RTSSystem {
    pub fn update_entities(&mut self, entities: &mut Vec<GameObject>, dt: f64, mode: &str) {
        let dt = dt.clamp(0.0, 0.1);
        if mode != "PLAY" {
            self.stats = self.collect_stats(entities, 0, 0, 0);
            return;
        }

        let gathered = self.update_gathering(entities, dt);
        let completed_constructions = self.update_construction(entities, dt);
        let produced = self.update_production(entities, dt);
        self.update_fog_of_war(entities);
        self.stats = self.collect_stats(entities, gathered, produced, completed_constructions);
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
