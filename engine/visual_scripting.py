class VisualScriptRuntime:
    def __init__(self, game):
        self.game = game
        self.global_variables = {}
        self.execution_limit = 128
        self.stats = {
            "graphs": 0,
            "nodes_executed": 0,
            "last_graph": None,
        }

    def update(self, dt):
        graphs = 0
        nodes = 0

        for entity in getattr(self.game.world, "entities", []):
            script = entity.get_component("VisualScript") if hasattr(entity, "get_component") else None

            if not script or not getattr(script, "enabled", True):
                continue

            if self.game.mode == "EDITOR" and not script.run_in_editor:
                continue

            graphs += 1

            if not getattr(script, "_started", False):
                nodes += self.execute_component(entity, script, "start")
                script._started = True

            nodes += self.execute_component(entity, script, "update", {"dt": dt})

        self.stats = {
            "graphs": graphs,
            "nodes_executed": nodes,
            "last_graph": self.stats.get("last_graph"),
        }

    def execute_graph(self, entity, graph_name, event_name):
        script = entity.get_component("VisualScript") if hasattr(entity, "get_component") else None

        if not script:
            return 0

        return self.execute_component(entity, script, event_name)

    def execute_component(self, entity, script, event_name, context=None):
        context = context or {}
        nodes = {node.get("id"): node for node in script.nodes}
        start = self.find_event_node(script.nodes, event_name)

        if not start:
            return 0

        current = start.get("next")
        executed = 0

        while current and executed < self.execution_limit:
            node = nodes.get(current)

            if not node:
                break

            executed += 1
            current = self.execute_node(entity, script, node, context)

        self.stats["last_graph"] = script.graph_name
        return executed

    def find_event_node(self, nodes, event_name):
        wanted = {
            "start": "EventStart",
            "update": "EventUpdate",
            "click": "EventClick",
            "collision": "EventCollision",
        }.get(event_name, event_name)

        for node in nodes:
            if node.get("type") == wanted:
                return node

        if event_name == "start":
            for node in nodes:
                if node.get("id") == "start":
                    return node

        return None

    def execute_node(self, entity, script, node, context):
        node_type = node.get("type")

        if node_type == "Log":
            self.log(str(node.get("message", "")))

        elif node_type == "SetVariable":
            script.variables[node.get("name", "value")] = node.get("value")

        elif node_type == "AddForce":
            body = entity.get_component("Rigidbody2D") if hasattr(entity, "get_component") else None

            if body:
                body.add_force(node.get("x", 0), node.get("y", 0), node.get("impulse", False))

        elif node_type == "Move":
            entity.x += float(node.get("x", 0))
            entity.y += float(node.get("y", 0))

            if hasattr(entity, "sync_to_components"):
                entity.sync_to_components()

        elif node_type == "SpawnPrefab":
            path = node.get("path")

            if path and hasattr(self.game, "prefab_manager"):
                self.game.prefab_manager.instantiate_prefab(self.game, path, entity.x, entity.y)

        elif node_type == "DestroySelf":
            if entity in getattr(self.game, "units", []):
                self.game.units.remove(entity)
                self.game.world.entities = self.game.units

        elif node_type == "SetText":
            ui = entity.get_component("UIElement") if hasattr(entity, "get_component") else None

            if ui:
                ui.text = str(node.get("text", ui.text))

        elif node_type == "AddComponent":
            api = getattr(self.game, "api", None)

            if api:
                api.add_component(entity, node.get("component", ""), node.get("data", {}))

        elif node_type == "SetBlackboard":
            api = getattr(self.game, "api", None)

            if api:
                api.set_blackboard(entity, node.get("name", "value"), node.get("value"))

        elif node_type == "EmitEvent":
            api = getattr(self.game, "api", None)

            if api:
                api.emit(node.get("event", "event"), {"entity": entity, "node": node})

        elif node_type == "Damage":
            api = getattr(self.game, "api", None)
            target = entity

            if node.get("target_id") and hasattr(self.game, "get_entity_by_id"):
                target = self.game.get_entity_by_id(node.get("target_id")) or entity

            if api:
                api.damage(target, node.get("amount", 1))

        elif node_type == "Heal":
            api = getattr(self.game, "api", None)

            if api:
                api.heal(entity, node.get("amount", 1))

        elif node_type == "AddItem":
            api = getattr(self.game, "api", None)

            if api:
                api.add_item(entity, node.get("item_id", "item"), node.get("quantity", 1))

        elif node_type == "StartCooldown":
            api = getattr(self.game, "api", None)

            if api:
                api.start_cooldown(entity, node.get("name", "cooldown"), node.get("duration", 1))

        elif node_type == "SetState":
            state_machine = entity.get_component("StateMachine") if hasattr(entity, "get_component") else None

            if state_machine:
                state_machine.set_state(node.get("state", state_machine.current_state))

        elif node_type == "Tween":
            api = getattr(self.game, "api", None)

            if api:
                api.tween(
                    entity,
                    node.get("property", "x"),
                    node.get("to", 0),
                    node.get("duration", 1),
                    node.get("easing", "smooth"),
                )

        elif node_type == "Branch":
            variable = node.get("variable")
            expected = node.get("equals", True)
            value = script.variables.get(variable, self.global_variables.get(variable))
            return node.get("true_next") if value == expected else node.get("false_next")

        elif node_type == "Compare":
            left = script.variables.get(node.get("left"), node.get("left_value", 0))
            right = script.variables.get(node.get("right"), node.get("right_value", 0))
            op = node.get("op", "==")
            result = False

            if op == "==":
                result = left == right
            elif op == "!=":
                result = left != right
            elif op == ">":
                result = float(left) > float(right)
            elif op == "<":
                result = float(left) < float(right)
            elif op == ">=":
                result = float(left) >= float(right)
            elif op == "<=":
                result = float(left) <= float(right)

            script.variables[node.get("result", "compare_result")] = result

        elif node_type == "Wait":
            timer_key = f"_wait_{node.get('id')}"
            current = float(script.variables.get(timer_key, 0.0))
            current += float(context.get("dt", 0.0))

            if current < float(node.get("duration", 1.0)):
                script.variables[timer_key] = current
                return None

            script.variables[timer_key] = 0.0

        elif node_type == "SetProgress":
            ui = entity.get_component("UIElement") if hasattr(entity, "get_component") else None

            if ui:
                ui.progress = float(node.get("value", ui.progress))
                ui.max_progress = float(node.get("max", ui.max_progress))

        elif node_type == "LoadScene":
            api = getattr(self.game, "api", None)

            if api:
                api.load_scene(node.get("scene", "main.scene"))

        elif node_type == "PlaySound":
            audio = entity.get_component("AudioSource") if hasattr(entity, "get_component") else None

            if audio:
                audio.audio_name = node.get("sound", audio.audio_name)
                audio.play_on_start = True
                audio._started = False

        return node.get("next")

    def log(self, message):
        if hasattr(self.game, "console"):
            self.game.console.log(f"VS: {message}", "SCRIPT")
