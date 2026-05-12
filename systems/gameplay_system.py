import math
import random

from entities.game_object import GameObject


class GameplaySystem:
    """
    Runtime gameplay layer for reusable game mechanics.
    """

    def __init__(self, game):
        self.game = game
        self.enabled = True
        self.run_in_editor = False
        self.run_in_play = True
        self.time = 0.0
        self.pending_destroy = []
        self.stats = {
            "lifetimes": 0,
            "nav_agents": 0,
            "ai_agents": 0,
            "spawners": 0,
            "interactions": 0,
            "destroyed": 0,
            "damage_events": 0,
        }

    def update(self, dt):
        dt = min(max(float(dt), 0.0), 0.05)
        self.time += dt
        self.pending_destroy = []
        damage_events = 0

        entities = list(getattr(self.game.world, "entities", []))

        for entity in entities:
            if not getattr(entity, "enabled", True):
                continue

            self.update_cooldown(entity, dt)
            self.update_timer(entity, dt)
            self.update_lifetime(entity, dt)
            self.update_status_effects(entity, dt)
            self.update_stat_regen(entity, dt)
            self.update_state_machine(entity, dt)
            self.update_tween(entity, dt)
            self.update_nav_agent(entity, dt)
            self.update_interaction(entity)
            self.update_character_controller(entity, dt)
            self.update_spawner(entity, dt)

        for entity in entities:
            if not getattr(entity, "enabled", True):
                continue

            damage_events += self.update_ai(entity, dt)

        self.update_camera_follow(entities, dt)
        destroyed = self.flush_destroy_queue()

        self.stats = {
            "lifetimes": self.count_components("Lifetime"),
            "nav_agents": self.count_components("NavAgent"),
            "ai_agents": self.count_components("AIController"),
            "spawners": self.count_components("Spawner"),
            "interactions": self.count_components("Interaction"),
            "destroyed": destroyed,
            "damage_events": damage_events,
        }

    def count_components(self, component_type):
        count = 0

        for entity in getattr(self.game.world, "entities", []):
            if self.component(entity, component_type):
                count += 1

        return count

    def component(self, entity, component_type):
        return entity.get_component(component_type) if hasattr(entity, "get_component") else None

    def entity_by_id(self, entity_id):
        if not entity_id:
            return None

        if hasattr(self.game, "get_entity_by_id"):
            return self.game.get_entity_by_id(entity_id)

        for entity in getattr(self.game.world, "entities", []):
            if getattr(entity, "id", None) == entity_id:
                return entity

        return None

    def distance(self, first, second):
        dx = float(getattr(first, "x", 0.0)) - float(getattr(second, "x", 0.0))
        dy = float(getattr(first, "y", 0.0)) - float(getattr(second, "y", 0.0))
        return math.hypot(dx, dy)

    def update_cooldown(self, entity, dt):
        cooldown = self.component(entity, "Cooldown")

        if cooldown:
            cooldown.tick(dt)

    def update_timer(self, entity, dt):
        timer = self.component(entity, "Timer")

        if not timer:
            return

        completed = timer.tick(dt)

        if completed and hasattr(self.game, "event_bus"):
            self.game.event_bus.emit(
                "timer_completed",
                {"entity": entity, "timer": timer.name},
            )

    def update_lifetime(self, entity, dt):
        lifetime = self.component(entity, "Lifetime")

        if not lifetime or lifetime.duration < 0:
            return

        lifetime.elapsed += dt

        if lifetime.elapsed >= lifetime.duration and lifetime.destroy_on_expire:
            self.destroy(entity)

    def update_status_effects(self, entity, dt):
        status = self.component(entity, "StatusEffects")

        if not status:
            return

        health = self.component(entity, "Health")

        for effect in list(status.effects):
            effect["elapsed"] = float(effect.get("elapsed", 0.0)) + dt
            stacks = max(1, int(effect.get("stacks", 1)))
            data = effect.get("data", {})

            if health and data.get("damage_per_second"):
                health.take_damage(float(data["damage_per_second"]) * dt * stacks)

            if health and data.get("heal_per_second"):
                health.heal(float(data["heal_per_second"]) * dt * stacks)

            if effect.get("duration", 0) >= 0 and effect["elapsed"] >= float(effect.get("duration", 0.0)):
                status.effects.remove(effect)

    def update_stat_regen(self, entity, dt):
        stats = self.component(entity, "Stats")
        health = self.component(entity, "Health")

        if stats and health and getattr(stats, "regen_per_second", 0) > 0:
            health.heal(float(stats.regen_per_second) * dt)

    def update_state_machine(self, entity, dt):
        machine = self.component(entity, "StateMachine")

        if not machine:
            return

        if machine.auto_start and not machine.current_state:
            machine.set_state(machine.initial_state)

        machine.time_in_state += dt
        blackboard = self.component(entity, "Blackboard")

        for transition in list(machine.transitions):
            if transition.get("from") not in (None, machine.current_state):
                continue

            after = transition.get("after")

            if after is not None and machine.time_in_state < float(after):
                continue

            key = transition.get("if")

            if key and blackboard:
                if blackboard.get(key) != transition.get("equals", True):
                    continue

            machine.set_state(transition.get("to", machine.current_state))
            break

    def update_tween(self, entity, dt):
        tween = self.component(entity, "Tween")

        if not tween or not tween.active:
            return

        tween.elapsed += dt
        self.set_property_path(entity, tween.property_path, tween.sample())

        if tween.elapsed < tween.duration:
            return

        if tween.loop:
            tween.elapsed = 0.0

            if tween.ping_pong:
                tween.from_value, tween.to_value = tween.to_value, tween.from_value
        else:
            tween.active = False

    def update_nav_agent(self, entity, dt):
        agent = self.component(entity, "NavAgent")

        if not agent or not agent.has_destination:
            return

        dx = float(agent.destination_x) - float(getattr(entity, "x", 0.0))
        dy = float(agent.destination_y) - float(getattr(entity, "y", 0.0))
        distance = math.hypot(dx, dy)

        if distance <= float(agent.stopping_distance):
            agent.clear_destination()
            setattr(entity, "path", [])
            setattr(entity, "state", "IDLE")
            return

        agent.repath_timer += dt
        can_use_path = hasattr(self.game, "command_system") and hasattr(entity, "path")

        if can_use_path and (not getattr(entity, "path", []) or agent.repath_timer >= agent.repath_interval):
            start = (int(getattr(entity, "x", 0)), int(getattr(entity, "y", 0)))
            goal = (int(agent.destination_x), int(agent.destination_y))
            path = self.game.command_system.build_path(start, goal)
            entity.path = path
            entity.command = "NAVIGATE"
            entity.state = "MOVING"
            agent.last_path_length = len(path)
            agent.repath_timer = 0.0
            return

        if not can_use_path and distance > 0:
            speed = float(getattr(agent, "speed", getattr(entity, "speed", 3.5)))
            step = min(distance, speed * dt)
            entity.x += dx / distance * step
            entity.y += dy / distance * step

            if hasattr(entity, "sync_to_components"):
                entity.sync_to_components()

    def update_interaction(self, entity):
        interaction = self.component(entity, "Interaction")

        if not interaction or (interaction.single_use and interaction.used):
            return

        interaction.active = False
        actor = self.find_nearest_by_tag(entity, interaction.requires_tag, interaction.radius)

        if not actor:
            return

        interaction.active = True
        input_map = getattr(self.game, "input_map", None)

        if not input_map or not input_map.get_action(interaction.action_name):
            return

        interaction.used = True
        visual_scripts = getattr(self.game, "visual_script_runtime", None)

        if visual_scripts and interaction.action_graph:
            visual_scripts.execute_graph(entity, interaction.action_graph, "interact")

        if hasattr(self.game, "event_bus"):
            self.game.event_bus.emit(
                "interaction",
                {"actor": actor, "target": entity, "prompt": interaction.prompt},
            )

    def update_character_controller(self, entity, dt):
        controller = self.component(entity, "CharacterController2D")

        if not controller or not controller.input_enabled:
            return

        if getattr(entity, "tag", "") != "Player":
            return

        input_map = getattr(self.game, "input_map", None)

        if not input_map:
            return

        body = self.component(entity, "Rigidbody2D")
        move = 0

        if input_map.get_action("move_left"):
            move -= 1
        if input_map.get_action("move_right"):
            move += 1

        speed = controller.run_speed if input_map.get_action("run") else controller.walk_speed
        target_speed = speed * move

        if body:
            body.velocity_x = target_speed

            if input_map.get_action("jump") and (controller.grounded or controller.jumps_used < controller.max_jumps):
                body.velocity_y = -abs(float(controller.jump_force))
                controller.jumps_used += 1
                controller.grounded = False
        else:
            entity.x += target_speed * dt

            if hasattr(entity, "sync_to_components"):
                entity.sync_to_components()

    def update_spawner(self, entity, dt):
        spawner = self.component(entity, "Spawner")

        if not spawner:
            return

        if getattr(self.game, "mode", "PLAY") == "EDITOR" and not spawner.enabled_in_editor:
            return

        spawner.spawned_ids = [
            entity_id for entity_id in spawner.spawned_ids
            if self.entity_by_id(entity_id)
        ]

        should_spawn = False

        if spawner.spawn_on_start and not spawner.started:
            should_spawn = True
            spawner.started = True

        spawner.elapsed += dt

        if spawner.elapsed >= spawner.spawn_interval:
            should_spawn = True
            spawner.elapsed = 0.0

        if not should_spawn or len(spawner.spawned_ids) >= int(spawner.max_alive):
            return

        spawned = self.spawn_from(entity, spawner)

        if spawned:
            spawner.spawned_ids.append(getattr(spawned, "id", ""))

    def spawn_from(self, entity, spawner):
        angle = random.random() * math.tau
        radius = random.random() * max(0.0, float(spawner.spawn_radius))
        x = float(getattr(entity, "x", 0.0)) + math.cos(angle) * radius
        y = float(getattr(entity, "y", 0.0)) + math.sin(angle) * radius

        prefab_name = str(spawner.prefab_name or "").strip()
        spawned = None

        if prefab_name and hasattr(self.game, "api"):
            try:
                spawned = self.game.api.instantiate(prefab_name, x, y)
            except Exception:
                spawned = None

        if not spawned:
            spawned = GameObject(x, y, self.game, name=prefab_name or "Spawned")
            self.game.units.append(spawned)
            self.game.world.entities = self.game.units

        return spawned

    def update_ai(self, entity, dt):
        ai = self.component(entity, "AIController")

        if not ai or ai.behavior == "idle":
            return 0

        ai.think_timer -= dt

        if ai.think_timer > 0:
            return 0

        ai.think_timer = max(0.02, float(ai.think_interval))
        target = self.entity_by_id(ai.target_id)

        if not target or self.distance(entity, target) > float(ai.leash_radius):
            target = self.find_nearest_target(entity, ai.target_tags, ai.detection_radius)
            ai.target_id = getattr(target, "id", None) if target else None

        if ai.behavior == "wander" and not target:
            self.wander(entity, ai)
            return 0

        if not target:
            ai.state = "idle"
            return 0

        distance = self.distance(entity, target)

        if ai.behavior in ("chase", "attack", "guard") and distance > float(ai.attack_radius):
            ai.state = "chase"
            self.move_entity_towards(entity, target)
            return 0

        if ai.behavior in ("attack", "guard"):
            ai.state = "attack"
            return self.attack(entity, target)

        return 0

    def wander(self, entity, ai):
        if getattr(entity, "path", []):
            return

        x = float(ai.home_x or getattr(entity, "x", 0.0))
        y = float(ai.home_y or getattr(entity, "y", 0.0))
        target = (
            int(x + random.uniform(-ai.wander_radius, ai.wander_radius)),
            int(y + random.uniform(-ai.wander_radius, ai.wander_radius)),
        )

        if hasattr(self.game, "command_system"):
            self.game.command_system.move_specific_unit_to(entity, target)

    def move_entity_towards(self, entity, target):
        if hasattr(self.game, "command_system") and hasattr(entity, "path"):
            if not getattr(entity, "path", []):
                self.game.command_system.move_specific_unit_to(
                    entity,
                    (int(getattr(target, "x", 0)), int(getattr(target, "y", 0))),
                )
            return

        nav = self.component(entity, "NavAgent")

        if nav:
            nav.set_destination(getattr(target, "x", 0), getattr(target, "y", 0))

    def attack(self, source, target):
        dealer = self.component(source, "DamageDealer")

        if not dealer:
            return 0

        target_id = getattr(target, "id", "")

        if not dealer.can_hit(target_id, self.time):
            return 0

        damage = float(dealer.damage)
        stats = self.component(source, "Stats")

        if stats:
            damage += stats.effective_attack() * 0.25

        target_stats = self.component(target, "Stats")

        if target_stats:
            damage = max(0.0, damage - target_stats.effective_defense() * 0.1)

        health = self.component(target, "Health")

        if health:
            health.take_damage(damage)

        dealer.mark_hit(target_id, self.time)

        if hasattr(self.game, "event_bus"):
            self.game.event_bus.emit(
                "damage",
                {"source": source, "target": target, "amount": damage},
            )

        if health and not health.alive:
            self.destroy(target)

        return 1

    def update_camera_follow(self, entities, dt):
        followers = [
            (entity, self.component(entity, "CameraFollow"))
            for entity in entities
            if self.component(entity, "CameraFollow")
        ]

        if not followers:
            return

        entity, follow = followers[0]
        target = self.entity_by_id(follow.target_id) if follow.target_id else entity
        viewport = self.game.get_world_viewport_rect() if hasattr(self.game, "get_world_viewport_rect") else None

        if not target or not viewport:
            return

        tile = max(1, getattr(getattr(self.game, "grid", None), "tile_size", 32))
        target_x = getattr(target, "x", 0.0) * tile + follow.offset_x
        target_y = getattr(target, "y", 0.0) * tile + follow.offset_y
        desired_x = target_x - viewport.width / 2 / max(0.001, self.game.camera.zoom)
        desired_y = target_y - viewport.height / 2 / max(0.001, self.game.camera.zoom)
        t = min(1.0, max(0.0, float(follow.smoothness)) * dt)

        if follow.follow_x:
            self.game.camera.x += (desired_x - self.game.camera.x) * t

        if follow.follow_y:
            self.game.camera.y += (desired_y - self.game.camera.y) * t

        self.game.camera.set_zoom(follow.zoom)
        self.game.camera.clamp()

    def find_nearest_by_tag(self, origin, tag, radius):
        if not tag:
            return None

        best = None
        best_distance = float(radius)

        for entity in getattr(self.game.world, "entities", []):
            if entity is origin or getattr(entity, "tag", None) != tag:
                continue

            distance = self.distance(origin, entity)

            if distance <= best_distance:
                best = entity
                best_distance = distance

        return best

    def find_nearest_target(self, origin, target_tags, radius):
        origin_team = self.component(origin, "Team")
        tags = set(target_tags or [])
        best = None
        best_distance = float(radius)

        for entity in getattr(self.game.world, "entities", []):
            if entity is origin or not getattr(entity, "enabled", True):
                continue

            team = self.component(entity, "Team")
            tagged = getattr(entity, "tag", None) in tags
            enemy = bool(origin_team and team and origin_team.is_enemy(team))

            if not tagged and not enemy:
                continue

            distance = self.distance(origin, entity)

            if distance <= best_distance:
                best = entity
                best_distance = distance

        return best

    def set_property_path(self, entity, path, value):
        if "." in path:
            component_type, attr = path.split(".", 1)
            component = self.component(entity, component_type)

            if component and hasattr(component, attr):
                setattr(component, attr, value)
                return True

        if hasattr(entity, path):
            setattr(entity, path, value)

            if hasattr(entity, "sync_to_components"):
                entity.sync_to_components()

            return True

        return False

    def destroy(self, entity):
        if entity not in self.pending_destroy:
            self.pending_destroy.append(entity)

    def flush_destroy_queue(self):
        destroyed = 0

        for entity in self.pending_destroy:
            if entity in getattr(self.game, "selected_units", []):
                self.game.selected_units.remove(entity)

            if entity in getattr(self.game, "units", []):
                self.game.units.remove(entity)
                destroyed += 1

        if destroyed:
            self.game.world.entities = self.game.units

        return destroyed
