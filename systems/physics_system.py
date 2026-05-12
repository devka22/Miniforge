import math


class PhysicsSystem:
    """
    Física 2D avanzada para MiniForge.

    Incluye Rigidbody2D, gravedad, drag, resolución AABB/círculo simplificada,
    matriz de capas y callbacks enter/stay/exit para collisions/triggers.
    """

    def __init__(self, game):
        self.game = game
        self.gravity = (0.0, 18.0)
        self.solver_iterations = 2
        self.active_pairs = {}
        self.layer_matrix = {}
        self.stats = {
            "bodies": 0,
            "colliders": 0,
            "pairs": 0,
            "contacts": 0,
        }

    def set_layer_collision(self, first_layer, second_layer, enabled):
        key = tuple(sorted((str(first_layer), str(second_layer))))
        self.layer_matrix[key] = bool(enabled)

    def layer_collision_enabled(self, first, second):
        first_layer = getattr(first, "layer", "Default")
        second_layer = getattr(second, "layer", "Default")
        key = tuple(sorted((str(first_layer), str(second_layer))))
        return self.layer_matrix.get(key, True)

    def update(self, dt):
        if getattr(self.game, "mode", "EDITOR") == "EDITOR":
            self.stats = {"bodies": 0, "colliders": 0, "pairs": 0, "contacts": 0}
            return

        entities = [
            entity for entity in getattr(self.game.world, "entities", [])
            if getattr(entity, "enabled", True)
            and getattr(entity, "visible", True)
        ]

        bodies = [entity for entity in entities if self.get_body(entity)]
        colliders = [entity for entity in entities if self.get_collider(entity)]

        self.integrate_bodies(bodies, dt)

        current_pairs = {}

        for _ in range(max(1, int(self.solver_iterations))):
            for index, first in enumerate(colliders):
                first_collider = self.get_collider(first)

                if not first_collider or not getattr(first_collider, "enabled", True):
                    continue

                for second in colliders[index + 1:]:
                    second_collider = self.get_collider(second)

                    if not second_collider or not getattr(second_collider, "enabled", True):
                        continue

                    if not self.layer_collision_enabled(first, second):
                        continue

                    contact = self.compute_contact(first, first_collider, second, second_collider)

                    if not contact["overlap"]:
                        continue

                    pair = self.pair_key(first, second)
                    is_trigger = (
                        getattr(first_collider, "is_trigger", False)
                        or getattr(second_collider, "is_trigger", False)
                    )
                    current_pairs[pair] = "trigger" if is_trigger else "collision"

                    if not is_trigger:
                        self.resolve_contact(first, second, contact)

        self.dispatch_pair_events(current_pairs)
        self.stats = {
            "bodies": len(bodies),
            "colliders": len(colliders),
            "pairs": len(current_pairs),
            "contacts": len(current_pairs),
        }

    def integrate_bodies(self, bodies, dt):
        dt = min(max(dt, 0.0), 0.05)
        gx, gy = self.gravity

        for entity in bodies:
            body = self.get_body(entity)

            if not body or not getattr(body, "enabled", True) or not body.is_dynamic:
                continue

            if body.use_gravity:
                body.velocity_x += gx * body.gravity_scale * dt
                body.velocity_y += gy * body.gravity_scale * dt

            damping = max(0.0, 1.0 - max(0.0, body.drag) * dt)
            body.velocity_x *= damping
            body.velocity_y *= damping

            if not body.freeze_x:
                entity.x += body.velocity_x * dt

            if not body.freeze_y:
                entity.y += body.velocity_y * dt

            if not body.freeze_rotation and hasattr(entity, "rotation"):
                angular_damping = max(0.0, 1.0 - max(0.0, body.angular_drag) * dt)
                body.angular_velocity *= angular_damping
                entity.rotation += body.angular_velocity * dt

            self.sync_entity(entity)

    def compute_contact(self, first, first_collider, second, second_collider):
        first_bounds = self.collider_bounds(first, first_collider)
        second_bounds = self.collider_bounds(second, second_collider)

        dx = first_bounds["cx"] - second_bounds["cx"]
        dy = first_bounds["cy"] - second_bounds["cy"]
        overlap_x = first_bounds["half_w"] + second_bounds["half_w"] - abs(dx)
        overlap_y = first_bounds["half_h"] + second_bounds["half_h"] - abs(dy)

        if overlap_x <= 0 or overlap_y <= 0:
            return {"overlap": False, "normal": (0, 0), "depth": 0}

        if overlap_x < overlap_y:
            normal = (1 if dx >= 0 else -1, 0)
            depth = overlap_x
        else:
            normal = (0, 1 if dy >= 0 else -1)
            depth = overlap_y

        return {"overlap": True, "normal": normal, "depth": depth}

    def collider_bounds(self, entity, collider):
        tile_size = max(1, getattr(getattr(self.game, "grid", None), "tile_size", 32))
        width = getattr(collider, "width", 1.0)
        height = getattr(collider, "height", 1.0)

        if getattr(collider, "shape", "rect") == "circle":
            radius = getattr(collider, "radius", 0.5)
            width = radius * 2.0
            height = radius * 2.0

        cx = getattr(entity, "x", 0.0) + getattr(collider, "offset_x", 0.0)
        cy = getattr(entity, "y", 0.0) + getattr(collider, "offset_y", 0.0)

        return {
            "cx": cx,
            "cy": cy,
            "half_w": max(0.05, float(width) * 0.5),
            "half_h": max(0.05, float(height) * 0.5),
            "tile_size": tile_size,
        }

    def resolve_contact(self, first, second, contact):
        first_body = self.get_body(first)
        second_body = self.get_body(second)

        first_dynamic = first_body and first_body.is_dynamic
        second_dynamic = second_body and second_body.is_dynamic

        if not first_dynamic and not second_dynamic:
            return

        nx, ny = contact["normal"]
        depth = contact["depth"] + 0.001

        first_share = 0.5 if first_dynamic and second_dynamic else 1.0
        second_share = 0.5 if first_dynamic and second_dynamic else 1.0

        if first_dynamic:
            first.x += nx * depth * first_share
            first.y += ny * depth * first_share
            self.apply_collision_velocity(first_body, nx, ny)
            self.sync_entity(first)

        if second_dynamic:
            second.x -= nx * depth * second_share
            second.y -= ny * depth * second_share
            self.apply_collision_velocity(second_body, -nx, -ny)
            self.sync_entity(second)

    def apply_collision_velocity(self, body, nx, ny):
        if not body:
            return

        velocity_dot = body.velocity_x * nx + body.velocity_y * ny

        if velocity_dot < 0:
            body.velocity_x -= (1.0 + body.bounciness) * velocity_dot * nx
            body.velocity_y -= (1.0 + body.bounciness) * velocity_dot * ny

        friction = max(0.0, min(1.0, body.friction))

        if abs(nx) > 0:
            body.velocity_y *= 1.0 - friction
        else:
            body.velocity_x *= 1.0 - friction

        if math.hypot(body.velocity_x, body.velocity_y) < 0.001:
            body.velocity_x = 0.0
            body.velocity_y = 0.0

    def dispatch_pair_events(self, current_pairs):
        previous_pairs = self.active_pairs

        for pair, pair_type in current_pairs.items():
            first, second = self.entities_from_pair(pair)

            if not first or not second:
                continue

            entered = pair not in previous_pairs
            callback = self.event_name(pair_type, "enter" if entered else "stay")
            self.dispatch(callback, first, second)
            self.dispatch(callback, second, first)

        for pair, pair_type in previous_pairs.items():
            if pair in current_pairs:
                continue

            first, second = self.entities_from_pair(pair)

            if not first or not second:
                continue

            callback = self.event_name(pair_type, "exit")
            self.dispatch(callback, first, second)
            self.dispatch(callback, second, first)

        self.active_pairs = current_pairs

    def event_name(self, pair_type, phase):
        if pair_type == "trigger":
            return f"on_trigger_{phase}"
        return f"on_collision_{phase}"

    def pair_key(self, first, second):
        return tuple(sorted((getattr(first, "id", ""), getattr(second, "id", ""))))

    def entities_from_pair(self, pair):
        return (self.get_entity(pair[0]), self.get_entity(pair[1]))

    def get_entity(self, entity_id):
        if hasattr(self.game, "get_entity_by_id"):
            return self.game.get_entity_by_id(entity_id)

        for entity in getattr(self.game, "units", []):
            if getattr(entity, "id", None) == entity_id:
                return entity

        return None

    def get_body(self, entity):
        return entity.get_component("Rigidbody2D") if hasattr(entity, "get_component") else None

    def get_collider(self, entity):
        return entity.get_component("Collider2D") if hasattr(entity, "get_component") else None

    def sync_entity(self, entity):
        if hasattr(entity, "sync_to_components"):
            entity.sync_to_components()

    def dispatch(self, callback, entity, other):
        targets = []
        targets.extend(getattr(entity, "components", []))
        targets.extend(getattr(entity, "scripts", []))

        for target in targets:
            method = getattr(target, callback, None)

            if not method:
                continue

            try:
                method(entity, other)
            except Exception as error:
                if hasattr(self.game, "console"):
                    self.game.console.log(
                        f"Physics callback error {callback}: {error}",
                        "ERROR"
                    )
