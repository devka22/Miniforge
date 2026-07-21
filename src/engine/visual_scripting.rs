use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::engine::component::default_component;
use crate::engine::game_api::GameAPI;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct VisualScriptRuntime {
    pub graphs: usize,
    pub last_frame_graphs: usize,
    pub executed_nodes: usize,
    pub logs: Vec<String>,
    pub last_errors: Vec<String>,
    started_entities: BTreeSet<u64>,
    graph_times: BTreeMap<u64, f64>,
}

impl VisualScriptRuntime {
    pub fn update_entities(&mut self, entities: &mut [GameObject], dt: f64, mode: &str) {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.1)
        } else {
            0.0
        };
        self.last_frame_graphs = 0;
        self.executed_nodes = 0;
        self.last_errors.clear();
        let live_entities = entities
            .iter()
            .map(|entity| entity.id)
            .collect::<BTreeSet<_>>();
        self.started_entities
            .retain(|entity_id| live_entities.contains(entity_id));
        self.graph_times
            .retain(|entity_id, _| live_entities.contains(entity_id));
        for entity in entities {
            let Some(script) = entity.get_component("VisualScript").cloned() else {
                continue;
            };
            if mode != "PLAY" && !script.get_bool("run_in_editor", false) {
                continue;
            }
            self.graphs += 1;
            self.last_frame_graphs += 1;
            let nodes = script
                .get("nodes")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if nodes.is_empty() {
                self.last_errors.push(format!(
                    "{}: VisualScript sin nodes; se omite este frame.",
                    entity.name
                ));
                continue;
            }
            let started = self.started_entities.contains(&entity.id);
            if !started {
                if graph_has_entry(&nodes, "construction", "ConstructionScript") {
                    self.execute_chain(entity, &nodes, "construction", dt);
                }
                self.execute_chain(entity, &nodes, "start", dt);
                self.started_entities.insert(entity.id);
            } else {
                self.execute_chain(entity, &nodes, "update", dt);
            }
        }
    }

    fn execute_chain(&mut self, entity: &mut GameObject, nodes: &[Value], start_id: &str, dt: f64) {
        const FRAME_NODE_BUDGET: usize = 4096;
        if self.executed_nodes >= FRAME_NODE_BUDGET {
            if !self
                .last_errors
                .iter()
                .any(|error| error.contains("presupuesto global"))
            {
                self.last_errors.push(format!(
                    "{}: VisualScript alcanzo el presupuesto global de {FRAME_NODE_BUDGET} nodos.",
                    entity.name
                ));
            }
            return;
        }
        let mut current = entry_node(nodes, start_id);
        let mut guard = 0;
        while let Some(node) = current {
            if self.executed_nodes >= FRAME_NODE_BUDGET {
                self.last_errors.push(format!(
                    "{}: VisualScript alcanzo el presupuesto global de {FRAME_NODE_BUDGET} nodos.",
                    entity.name
                ));
                break;
            }
            guard += 1;
            if guard > 128 {
                self.last_errors.push(format!(
                    "{}: VisualScript detenido por limite de 128 nodos.",
                    entity.name
                ));
                break;
            }
            self.executed_nodes += 1;
            let mut next_override = None;
            match node.get("type").and_then(Value::as_str).unwrap_or("") {
                "" => self
                    .last_errors
                    .push(format!("{}: nodo sin type en VisualScript.", entity.name)),
                "EventStart" | "EventUpdate" | "EventClick" | "EventTrigger"
                | "ConstructionScript" | "CustomEvent" => {}
                "CallEvent" => {
                    let event = node.get("event").and_then(Value::as_str).unwrap_or("");
                    next_override = nodes
                        .iter()
                        .find(|candidate| {
                            candidate.get("id").and_then(Value::as_str) == Some(event)
                                || candidate.get("event").and_then(Value::as_str) == Some(event)
                        })
                        .and_then(|candidate| candidate.get("id").and_then(Value::as_str))
                        .map(ToString::to_string);
                }
                "BroadcastEvent" => {
                    let event = node.get("event").and_then(Value::as_str).unwrap_or("");
                    let targets = custom_event_targets(nodes, event);
                    for target in targets {
                        self.execute_chain(entity, nodes, &target, dt);
                    }
                }
                "Sequence" => {
                    for pin in ["then_0", "then_1"] {
                        if let Some(target) = node.get(pin).and_then(Value::as_str) {
                            self.execute_chain(entity, nodes, target, dt);
                        }
                    }
                }
                "DoOnce" => {
                    let key = do_once_key(node);
                    if graph_variable(entity, &key)
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false)
                    {
                        break;
                    }
                    set_graph_variable(entity, &key, serde_json::json!(true));
                }
                "ResetDoOnce" => {
                    let key = do_once_key(node);
                    set_graph_variable(entity, &key, serde_json::json!(false));
                }
                "Gate" => {
                    let key = gate_key(node);
                    let open = graph_variable(entity, &key)
                        .and_then(|value| value.as_bool())
                        .unwrap_or_else(|| {
                            node.get("open").and_then(Value::as_bool).unwrap_or(true)
                        });
                    if !open {
                        break;
                    }
                }
                "OpenGate" => {
                    set_graph_variable(entity, &gate_key(node), serde_json::json!(true));
                }
                "CloseGate" => {
                    set_graph_variable(entity, &gate_key(node), serde_json::json!(false));
                }
                "ToggleGate" => {
                    let key = gate_key(node);
                    let current = graph_variable(entity, &key)
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    set_graph_variable(entity, &key, serde_json::json!(!current));
                }
                "FlipFlop" => {
                    let key = flip_flop_key(node);
                    let use_b = graph_variable(entity, &key)
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    set_graph_variable(entity, &key, serde_json::json!(!use_b));
                    let pin = if use_b { "b_next" } else { "a_next" };
                    next_override = node
                        .get(pin)
                        .and_then(Value::as_str)
                        .map(ToString::to_string);
                }
                "Move" => {
                    let scale = if node.get("use_dt").and_then(Value::as_bool).unwrap_or(false) {
                        dt
                    } else {
                        1.0
                    };
                    entity.x += node.get("x").and_then(Value::as_f64).unwrap_or(0.0) * scale;
                    entity.y += node.get("y").and_then(Value::as_f64).unwrap_or(0.0) * scale;
                    entity.sync_to_components();
                }
                "MoveTowards" => {
                    let target_x = node
                        .get("target_x")
                        .and_then(Value::as_f64)
                        .unwrap_or(entity.x);
                    let target_y = node
                        .get("target_y")
                        .and_then(Value::as_f64)
                        .unwrap_or(entity.y);
                    let speed = node
                        .get("speed")
                        .and_then(Value::as_f64)
                        .unwrap_or(entity.speed);
                    let dx = target_x - entity.x;
                    let dy = target_y - entity.y;
                    let distance = (dx * dx + dy * dy).sqrt();
                    if distance > 0.0001 {
                        let step = (speed * dt).min(distance);
                        entity.x += (dx / distance) * step;
                        entity.y += (dy / distance) * step;
                        entity.sync_to_components();
                    }
                }
                "SetVelocity" => {
                    ensure_component(entity, "Rigidbody2D");
                    if let Some(body) = entity.get_component_mut("Rigidbody2D") {
                        body.set_f64(
                            "velocity_x",
                            node.get("x").and_then(Value::as_f64).unwrap_or(0.0),
                        );
                        body.set_f64(
                            "velocity_y",
                            node.get("y").and_then(Value::as_f64).unwrap_or(0.0),
                        );
                        body.set("sleeping", serde_json::json!(false));
                    }
                }
                "AddForce" => {
                    ensure_component(entity, "Rigidbody2D");
                    if let Some(body) = entity.get_component_mut("Rigidbody2D") {
                        body.add_force(
                            node.get("x").and_then(Value::as_f64).unwrap_or(0.0),
                            node.get("y").and_then(Value::as_f64).unwrap_or(0.0),
                            node.get("impulse").and_then(Value::as_bool).unwrap_or(true),
                        );
                    }
                }
                "StopMovement" => {
                    entity.command = "STOP".to_string();
                    entity.path.clear();
                    entity.state = "IDLE".to_string();
                    if let Some(body) = entity.get_component_mut("Rigidbody2D") {
                        body.set_f64("velocity_x", 0.0);
                        body.set_f64("velocity_y", 0.0);
                    }
                }
                "SetSpeed" => {
                    let speed = node
                        .get("speed")
                        .and_then(Value::as_f64)
                        .unwrap_or(entity.speed);
                    entity.speed = speed;
                    if let Some(movement) = entity.get_component_mut("RTSMovement") {
                        movement.set_f64("speed", speed);
                    }
                    if let Some(controller) = entity.get_component_mut("CharacterController2D") {
                        controller.set_f64("walk_speed", speed);
                    }
                }
                "SetPosition" => {
                    entity.x = node.get("x").and_then(Value::as_f64).unwrap_or(entity.x);
                    entity.y = node.get("y").and_then(Value::as_f64).unwrap_or(entity.y);
                    entity.sync_to_components();
                }
                "SetRotation" => {
                    entity.rotation = node
                        .get("rotation")
                        .and_then(Value::as_f64)
                        .unwrap_or(entity.rotation);
                    entity.sync_to_components();
                }
                "SetScale" => {
                    entity.scale_x = node
                        .get("x")
                        .and_then(Value::as_f64)
                        .unwrap_or(entity.scale_x);
                    entity.scale_y = node
                        .get("y")
                        .and_then(Value::as_f64)
                        .unwrap_or(entity.scale_y);
                    entity.sync_to_components();
                }
                "Log" => {
                    self.logs.push(
                        node.get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("Visual script running")
                            .to_string(),
                    );
                }
                "Damage" => {
                    ensure_component(entity, "Health");
                    if let Some(health) = entity.get_component_mut("Health") {
                        health
                            .take_damage(node.get("amount").and_then(Value::as_f64).unwrap_or(0.0));
                    }
                }
                "Heal" => {
                    ensure_component(entity, "Health");
                    if let Some(health) = entity.get_component_mut("Health") {
                        health.heal(node.get("amount").and_then(Value::as_f64).unwrap_or(0.0));
                    }
                }
                "SetHealth" => {
                    ensure_component(entity, "Health");
                    if let Some(health) = entity.get_component_mut("Health") {
                        let max_health = node
                            .get("max_health")
                            .and_then(Value::as_f64)
                            .unwrap_or_else(|| health.get_f64("max_health", 100.0));
                        let value = node
                            .get("health")
                            .and_then(Value::as_f64)
                            .unwrap_or(max_health)
                            .clamp(0.0, max_health);
                        health.set_f64("max_health", max_health);
                        health.set_f64("health", value);
                        health.set("alive", serde_json::json!(value > 0.0));
                    }
                }
                "BranchHealth" => {
                    let health = entity
                        .get_component("Health")
                        .map(|health| health.get_f64("health", health.get_f64("max_health", 100.0)))
                        .unwrap_or(0.0);
                    let target = node.get("value").and_then(Value::as_f64).unwrap_or(0.0);
                    let passed = compare_numbers(
                        health,
                        target,
                        node.get("operator").and_then(Value::as_str).unwrap_or("<="),
                    );
                    next_override = branch_next(node, passed);
                }
                "BranchVariable" => {
                    let variable_name = node.get("name").and_then(Value::as_str).unwrap_or("");
                    let current = graph_variable(entity, variable_name).unwrap_or(Value::Null);
                    let passed = compare_values(
                        &current,
                        node.get("value").unwrap_or(&Value::Bool(true)),
                        node.get("operator").and_then(Value::as_str).unwrap_or("=="),
                    );
                    next_override = branch_next(node, passed);
                }
                "SetEnabled" => {
                    entity.enabled = node
                        .get("value")
                        .and_then(Value::as_bool)
                        .unwrap_or(entity.enabled);
                    entity.active = entity.enabled;
                }
                "SetTag" => {
                    entity.tag = node
                        .get("tag")
                        .and_then(Value::as_str)
                        .unwrap_or("Untagged")
                        .to_string();
                }
                "SetVariable" => {
                    if let Some(name) = node.get("name").and_then(Value::as_str) {
                        set_graph_variable(
                            entity,
                            name,
                            node.get("value").cloned().unwrap_or(Value::Null),
                        );
                    }
                }
                "AddVariable" => {
                    if let Some(name) = node.get("name").and_then(Value::as_str) {
                        let current = graph_variable(entity, name)
                            .and_then(|value| value.as_f64())
                            .unwrap_or(0.0);
                        let amount = node.get("amount").and_then(Value::as_f64).unwrap_or(1.0);
                        set_graph_variable(entity, name, serde_json::json!(current + amount));
                    }
                }
                "ToggleVariable" => {
                    if let Some(name) = node.get("name").and_then(Value::as_str) {
                        let current = graph_variable(entity, name)
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false);
                        set_graph_variable(entity, name, serde_json::json!(!current));
                    }
                }
                "SetBlackboard" => {
                    ensure_component(entity, "Blackboard");
                    let key = node.get("key").and_then(Value::as_str).unwrap_or("value");
                    let value = node.get("value").cloned().unwrap_or(Value::Null);
                    if let Some(blackboard) = entity.get_component_mut("Blackboard") {
                        blackboard.blackboard_set(key, value);
                    }
                }
                "Wait" => {
                    let seconds = node.get("seconds").and_then(Value::as_f64).unwrap_or(0.0);
                    if seconds > 0.0 {
                        let id = node.get("id").and_then(Value::as_str).unwrap_or("wait");
                        let key = format!("__wait_{id}");
                        let elapsed = graph_variable(entity, &key)
                            .and_then(|value| value.as_f64())
                            .unwrap_or(0.0)
                            + dt.max(0.0);
                        if elapsed < seconds {
                            set_graph_variable(entity, &key, serde_json::json!(elapsed));
                            break;
                        }
                        set_graph_variable(entity, &key, serde_json::json!(0.0));
                    }
                }
                "ConfigureSpawner" => {
                    ensure_component(entity, "Spawner");
                    if let Some(spawner) = entity.get_component_mut("Spawner") {
                        if let Some(prefab) = node.get("prefab").and_then(Value::as_str) {
                            spawner.set("prefab_name", serde_json::json!(prefab));
                        }
                        if let Some(interval) = node.get("interval").and_then(Value::as_f64) {
                            spawner.set_f64("spawn_interval", interval.max(0.01));
                        }
                        if let Some(radius) = node.get("radius").and_then(Value::as_f64) {
                            spawner.set_f64("spawn_radius", radius.max(0.0));
                        }
                        if let Some(max_alive) = node.get("max_alive").and_then(Value::as_i64) {
                            spawner.set("max_alive", serde_json::json!(max_alive.max(0)));
                        }
                        if let Some(spawn_on_start) =
                            node.get("spawn_on_start").and_then(Value::as_bool)
                        {
                            spawner.set("spawn_on_start", serde_json::json!(spawn_on_start));
                        }
                    }
                }
                "SetAnimation" => {
                    ensure_component(entity, "Animator");
                    if let Some(animator) = entity.get_component_mut("Animator") {
                        animator.set(
                            "current_state",
                            serde_json::json!(
                                node.get("state").and_then(Value::as_str).unwrap_or("Idle")
                            ),
                        );
                    }
                }
                "SetUiText" => {
                    ensure_component(entity, "UIElement");
                    if let Some(ui) = entity.get_component_mut("UIElement") {
                        ui.set(
                            "text",
                            serde_json::json!(
                                node.get("text").and_then(Value::as_str).unwrap_or("Ready")
                            ),
                        );
                    }
                }
                "InventoryAdd" => {
                    ensure_component(entity, "Inventory");
                    if let Some(inventory) = entity.get_component_mut("Inventory") {
                        let item = node.get("item").and_then(Value::as_str).unwrap_or("item");
                        let quantity = node.get("quantity").and_then(Value::as_i64).unwrap_or(1);
                        inventory.inventory_add_item(item, quantity, serde_json::json!({}));
                    }
                }
                "UseInventoryItem" => {
                    let item = node.get("item").and_then(Value::as_str).unwrap_or("item");
                    let used = GameAPI::use_item(entity, item);
                    if node.get("true_next").is_some() || node.get("false_next").is_some() {
                        next_override = branch_next(node, used);
                    }
                }
                "SortInventory" => {
                    let mode = node.get("mode").and_then(Value::as_str).unwrap_or("id");
                    let _ = GameAPI::sort_inventory(entity, mode);
                }
                "SetSurvivalNeed" => {
                    let need = node.get("need").and_then(Value::as_str).unwrap_or("hunger");
                    let value = node.get("value").and_then(Value::as_f64).unwrap_or(100.0);
                    let _ = GameAPI::set_survival_need(entity, need, value);
                }
                "ModifySurvivalNeed" => {
                    let need = node.get("need").and_then(Value::as_str).unwrap_or("hunger");
                    let delta = node.get("delta").and_then(Value::as_f64).unwrap_or(0.0);
                    let _ = GameAPI::modify_survival_need(entity, need, delta);
                }
                "BranchSurvivalNeed" => {
                    let need = node.get("need").and_then(Value::as_str).unwrap_or("hunger");
                    let current = GameAPI::survival_need(entity, need).unwrap_or(0.0);
                    let target = node.get("value").and_then(Value::as_f64).unwrap_or(0.0);
                    let passed = compare_numbers(
                        current,
                        target,
                        node.get("operator").and_then(Value::as_str).unwrap_or("<="),
                    );
                    next_override = branch_next(node, passed);
                }
                "CraftRecipe" => {
                    let recipe = node
                        .get("recipe")
                        .and_then(Value::as_str)
                        .unwrap_or("recipe");
                    let crafted = GameAPI::craft(entity, recipe).crafted;
                    if node.get("true_next").is_some() || node.get("false_next").is_some() {
                        next_override = branch_next(node, crafted);
                    }
                }
                "InventoryRemove" => {
                    let item = node.get("item").and_then(Value::as_str).unwrap_or("item");
                    let quantity = node.get("quantity").and_then(Value::as_i64).unwrap_or(1);
                    let removed = GameAPI::remove_item(entity, item, quantity);
                    if node.get("true_next").is_some() || node.get("false_next").is_some() {
                        next_override = branch_next(node, removed >= quantity.max(1));
                    }
                }
                "BranchItem" => {
                    let item = node.get("item").and_then(Value::as_str).unwrap_or("item");
                    let quantity = node.get("quantity").and_then(Value::as_i64).unwrap_or(1);
                    next_override = branch_next(node, GameAPI::has_item(entity, item, quantity));
                }
                "EquipItem" => {
                    let slot = node.get("slot").and_then(Value::as_str).unwrap_or("weapon");
                    let item = node.get("item").and_then(Value::as_str).unwrap_or("item");
                    let bonuses = node
                        .get("bonuses")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!({}));
                    GameAPI::equip_item(entity, slot, item, bonuses);
                }
                "EconomyAdd" => {
                    let resource = node
                        .get("resource")
                        .and_then(Value::as_str)
                        .unwrap_or("Gold");
                    let amount = node.get("amount").and_then(Value::as_f64).unwrap_or(0.0);
                    let _ = GameAPI::add_resource(entity, resource, amount);
                }
                "EconomySpend" => {
                    let resource = node
                        .get("resource")
                        .and_then(Value::as_str)
                        .unwrap_or("Gold");
                    let amount = node.get("amount").and_then(Value::as_f64).unwrap_or(0.0);
                    let spent = GameAPI::spend_resource(entity, resource, amount);
                    if node.get("true_next").is_some() || node.get("false_next").is_some() {
                        next_override = branch_next(node, spent);
                    } else if !spent {
                        break;
                    }
                }
                "BranchResource" => {
                    let resource = node
                        .get("resource")
                        .and_then(Value::as_str)
                        .unwrap_or("Gold");
                    let amount = node.get("amount").and_then(Value::as_f64).unwrap_or(0.0);
                    next_override =
                        branch_next(node, GameAPI::resource_amount(entity, resource) >= amount);
                }
                "AddProductionRecipe" => {
                    let unit = node.get("unit").and_then(Value::as_str).unwrap_or("Worker");
                    let display = node.get("display").and_then(Value::as_str).unwrap_or(unit);
                    let build_time = node
                        .get("build_time")
                        .and_then(Value::as_f64)
                        .unwrap_or(3.0);
                    let cost = node
                        .get("cost")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!({}));
                    GameAPI::add_production_recipe(entity, unit, display, build_time, cost);
                }
                "SetPreferredRecipe" => {
                    let unit = node.get("unit").and_then(Value::as_str).unwrap_or("Worker");
                    GameAPI::set_preferred_recipe(entity, unit);
                }
                "QueuePreferredRecipe" => {
                    GameAPI::enqueue_preferred_recipe(entity);
                }
                "AddQuest" => {
                    let quest = node.get("quest").and_then(Value::as_str).unwrap_or("quest");
                    let title = node.get("title").and_then(Value::as_str).unwrap_or(quest);
                    let objectives = node
                        .get("objectives")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!([]));
                    GameAPI::add_quest(entity, quest, title, objectives);
                }
                "QuestProgress" => {
                    let quest = node.get("quest").and_then(Value::as_str).unwrap_or("quest");
                    let objective = node
                        .get("objective")
                        .and_then(Value::as_str)
                        .unwrap_or("objective");
                    let progress = node
                        .get("progress")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!(1));
                    GameAPI::set_quest_objective_progress(entity, quest, objective, progress);
                }
                "TriggerAbility" => {
                    let now = self.advance_graph_time(entity.id, dt);
                    let fired = GameAPI::trigger_ability(entity, now);
                    if node.get("true_next").is_some() || node.get("false_next").is_some() {
                        next_override = branch_next(node, fired);
                    }
                }
                "RechargeAbility" => {
                    GameAPI::recharge_ability(
                        entity,
                        node.get("amount").and_then(Value::as_i64).unwrap_or(1),
                    );
                }
                "StartCooldown" => {
                    ensure_component(entity, "Cooldown");
                    if let Some(cooldown) = entity.get_component_mut("Cooldown") {
                        cooldown.cooldown_start(
                            node.get("name")
                                .and_then(Value::as_str)
                                .unwrap_or("cooldown"),
                            node.get("duration").and_then(Value::as_f64).unwrap_or(1.0),
                        );
                    }
                }
                "SetState" => {
                    let state = node
                        .get("state")
                        .and_then(Value::as_str)
                        .unwrap_or("Active")
                        .to_string();
                    entity.state = state.clone();
                    if let Some(machine) = entity.get_component_mut("StateMachine") {
                        machine.state_machine_set_state(&state);
                    }
                }
                "AddStatusEffect" => {
                    ensure_component(entity, "StatusEffects");
                    if let Some(status) = entity.get_component_mut("StatusEffects") {
                        status.status_add_effect(
                            node.get("name").and_then(Value::as_str).unwrap_or("Effect"),
                            node.get("duration").and_then(Value::as_f64).unwrap_or(1.0),
                            node.get("stacks").and_then(Value::as_i64).unwrap_or(1),
                            serde_json::json!({
                                "damage_per_second": node.get("damage_per_second").and_then(Value::as_f64).unwrap_or(0.0),
                                "heal_per_second": node.get("heal_per_second").and_then(Value::as_f64).unwrap_or(0.0),
                            }),
                        );
                    }
                }
                "CompleteQuest" => {
                    ensure_component(entity, "QuestLog");
                    if let Some(quest_log) = entity.get_component_mut("QuestLog") {
                        let quest = node.get("quest").and_then(Value::as_str).unwrap_or("quest");
                        quest_log.quest_complete(quest);
                    }
                }
                "AddComponent" => {
                    let component = node
                        .get("component")
                        .and_then(Value::as_str)
                        .unwrap_or("Health");
                    ensure_component(entity, component);
                }
                "SetComponentNumber" => {
                    let component = node
                        .get("component")
                        .and_then(Value::as_str)
                        .unwrap_or("Stats");
                    let field = node.get("field").and_then(Value::as_str).unwrap_or("value");
                    let value = node.get("value").and_then(Value::as_f64).unwrap_or(0.0);
                    ensure_component(entity, component);
                    if let Some(component) = entity.get_component_mut(component) {
                        component.set_f64(field, value);
                    }
                }
                "DestroySelf" => {
                    entity.enabled = false;
                    entity.active = false;
                    entity.visible = false;
                }
                other => self.last_errors.push(format!(
                    "{}: nodo VisualScript desconocido: {other}",
                    entity.name
                )),
            }
            let next = next_override
                .as_deref()
                .or_else(|| node.get("next").and_then(Value::as_str));
            current = next.and_then(|id| {
                let found = nodes
                    .iter()
                    .find(|node| node.get("id").and_then(Value::as_str) == Some(id));
                if found.is_none() {
                    self.last_errors.push(format!(
                        "{}: next apunta a nodo inexistente: {id}",
                        entity.name
                    ));
                }
                found
            });
        }
    }

    fn advance_graph_time(&mut self, entity_id: u64, dt: f64) -> f64 {
        let time = self.graph_times.entry(entity_id).or_default();
        *time += dt.max(0.0);
        *time
    }
}

fn ensure_component(entity: &mut GameObject, component_type: &str) {
    if entity.get_component(component_type).is_none()
        && let Some(component) = default_component(component_type)
    {
        entity.add_component(component);
    }
}

fn graph_has_entry(nodes: &[Value], id: &str, node_type: &str) -> bool {
    nodes.iter().any(|node| {
        node.get("id").and_then(Value::as_str) == Some(id)
            || node.get("type").and_then(Value::as_str) == Some(node_type)
    })
}

fn entry_node<'a>(nodes: &'a [Value], start_id: &str) -> Option<&'a Value> {
    nodes
        .iter()
        .find(|node| node.get("id").and_then(Value::as_str) == Some(start_id))
        .or_else(|| match start_id {
            "construction" => nodes.iter().find(|node| {
                node.get("type").and_then(Value::as_str) == Some("ConstructionScript")
            }),
            "update" => nodes
                .iter()
                .find(|node| node.get("type").and_then(Value::as_str) == Some("EventUpdate"))
                .or_else(|| {
                    nodes
                        .iter()
                        .find(|node| node.get("type").and_then(Value::as_str) == Some("EventStart"))
                }),
            _ => nodes
                .iter()
                .find(|node| node.get("type").and_then(Value::as_str) == Some("EventStart")),
        })
}

fn do_once_key(node: &Value) -> String {
    format!(
        "__do_once_{}",
        node.get("key")
            .and_then(Value::as_str)
            .or_else(|| node.get("id").and_then(Value::as_str))
            .unwrap_or("default")
    )
}

fn gate_key(node: &Value) -> String {
    format!(
        "__gate_{}",
        node.get("key")
            .and_then(Value::as_str)
            .or_else(|| node.get("id").and_then(Value::as_str))
            .unwrap_or("main")
    )
}

fn flip_flop_key(node: &Value) -> String {
    format!(
        "__flipflop_{}",
        node.get("key")
            .and_then(Value::as_str)
            .or_else(|| node.get("id").and_then(Value::as_str))
            .unwrap_or("main")
    )
}

fn custom_event_targets(nodes: &[Value], event: &str) -> Vec<String> {
    nodes
        .iter()
        .filter(|node| node.get("type").and_then(Value::as_str) == Some("CustomEvent"))
        .filter(|node| {
            node.get("event").and_then(Value::as_str) == Some(event)
                || node.get("id").and_then(Value::as_str) == Some(event)
        })
        .filter_map(|node| {
            node.get("id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .collect()
}

fn graph_variable(entity: &GameObject, name: &str) -> Option<Value> {
    entity
        .get_component("VisualScript")?
        .get("variables")?
        .as_object()?
        .get(name)
        .cloned()
}

fn set_graph_variable(entity: &mut GameObject, name: &str, value: Value) {
    let Some(script) = entity.get_component_mut("VisualScript") else {
        return;
    };
    let mut vars = script
        .get("variables")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    vars.insert(name.to_string(), value);
    script.set("variables", Value::Object(vars));
}

fn branch_next(node: &Value, passed: bool) -> Option<String> {
    let key = if passed { "true_next" } else { "false_next" };
    node.get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn compare_values(current: &Value, target: &Value, operator: &str) -> bool {
    if let (Some(left), Some(right)) = (current.as_f64(), target.as_f64()) {
        return compare_numbers(left, right, operator);
    }
    match operator {
        "!=" => current != target,
        "contains" => current
            .as_str()
            .zip(target.as_str())
            .is_some_and(|(left, right)| left.contains(right)),
        _ => current == target,
    }
}

fn compare_numbers(left: f64, right: f64, operator: &str) -> bool {
    match operator {
        ">" => left > right,
        ">=" => left >= right,
        "<" => left < right,
        "<=" => left <= right,
        "!=" => (left - right).abs() > f64::EPSILON,
        _ => (left - right).abs() <= f64::EPSILON,
    }
}
