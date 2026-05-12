from engine.animation import AnimationController, ColorAnimation
from engine.entity_id import generate_entity_id, generate_entity_name
from engine.script import invoke_script_method

from engine.component import (
    Transform,
    SpriteRenderer,
    RTSMovement,
    Selectable,
    Collider2D,
)


class Unit:
    def __init__(self, x, y, game, entity_id=None, name=None):
        self.game = game

        self.id = entity_id if entity_id else generate_entity_id()
        self.name = name if name else generate_entity_name("Unit")

        self.enabled = True
        self.active = True
        self.visible = True
        self.locked = False

        self.x = float(x)
        self.y = float(y)
        self.rotation = 0.0
        self.scale_x = 1.0
        self.scale_y = 1.0
        self.width = 1.0
        self.height = 1.0

        self.selected = False
        self.path = []

        self.speed = 4.5
        self.radius = 0.45

        self.state = "IDLE"
        self.command = "IDLE"

        self.tag = "Untagged"
        self.layer = "Default"

        self.prefab_source = None
        self.prefab_guid = None
        self.is_prefab_instance = False
        self.parent_id = None
        self.local_x = 0.0
        self.local_y = 0.0
        self.sprite_guid = None

        self.patrol_points = []
        self.patrol_index = 0

        self.follow_target_id = None
        self.guard_target_id = None
        self.attack_move_target = None
        self.gather_target_id = None

        self.components = []
        self.scripts = []

        self.sprite_name = None
        self.script = None
        self.debug_color_flip = False

        self.animator = AnimationController()

        self.animator.add(
            "IDLE",
            ColorAnimation([(0, 120, 255), (0, 140, 255)], speed=0.6)
        )

        self.animator.add(
            "MOVING",
            ColorAnimation([(0, 200, 255), (0, 255, 200)], speed=0.15)
        )

        self.animator.add(
            "SELECTED",
            ColorAnimation([(0, 255, 0), (120, 255, 120)], speed=0.2)
        )

        self.animator.add(
            "LOCKED",
            ColorAnimation([(120, 120, 120), (90, 90, 90)], speed=0.4)
        )

        self.add_component(Transform(self.x, self.y))
        self.add_component(Selectable(True))
        self.add_component(RTSMovement(self.speed))
        self.add_component(SpriteRenderer(None))
        self.add_component(Collider2D("rect", 1.0, 1.0, self.radius))

    # =========================
    # COMPONENTS
    # =========================

    def add_component(self, component):
        if not component:
            return None

        existing = self.get_component(component.component_type)

        if existing:
            return existing

        self.components.append(component)

        if hasattr(component, "start"):
            component.start(self)

        return component

    def get_component(self, component_type):
        for component in self.components:
            if component.component_type == component_type:
                return component

        return None

    def remove_component(self, component_type):
        self.components = [
            component for component in self.components
            if component.component_type != component_type
        ]

    # =========================
    # SCRIPTS
    # =========================

    def add_script(self, script):
        if not script:
            return

        self.scripts.append(script)

        if hasattr(script, "start"):
            invoke_script_method(script, "start", self)

    def set_selected(self, value):
        if self.selected == value:
            return

        self.selected = value

        callback = "on_selected" if value else "on_deselected"

        for script in self.scripts:
            if hasattr(script, callback):
                try:
                    invoke_script_method(script, callback, self)
                except Exception as error:
                    self.game.console.log(f"Script selection error: {error}", "SCRIPT")

    # =========================
    # UPDATE
    # =========================

    def update(self, dt=0.016):
        if not self.enabled:
            return

        self.sync_from_components()
        self.update_components(dt)
        self.update_scripts(dt)
        self.update_movement(dt)
        self.update_follow_guard()
        self.update_gather()
        self.apply_separation()
        self.update_animation(dt)
        self.sync_to_components()

    def sync_from_components(self):
        transform = self.get_component("Transform")

        if transform:
            self.x = transform.x
            self.y = transform.y
            self.rotation = getattr(transform, "rotation", self.rotation)
            self.scale_x = getattr(transform, "scale_x", self.scale_x)
            self.scale_y = getattr(transform, "scale_y", self.scale_y)

        movement = self.get_component("RTSMovement")

        if movement:
            self.speed = movement.speed

        sprite = self.get_component("SpriteRenderer")

        if sprite:
            self.sprite_name = sprite.sprite_name

        collider = self.get_component("Collider2D")

        if collider:
            self.radius = getattr(collider, "radius", self.radius)
            self.width = getattr(collider, "width", self.width)
            self.height = getattr(collider, "height", self.height)

    def sync_to_components(self):
        transform = self.get_component("Transform")

        if transform:
            transform.x = self.x
            transform.y = self.y
            transform.rotation = self.rotation
            transform.scale_x = self.scale_x
            transform.scale_y = self.scale_y

        movement = self.get_component("RTSMovement")

        if movement:
            movement.speed = self.speed

        sprite = self.get_component("SpriteRenderer")

        if sprite:
            sprite.sprite_name = self.sprite_name

        collider = self.get_component("Collider2D")

        if collider:
            collider.radius = self.radius
            collider.width = self.width
            collider.height = self.height

    def update_components(self, dt):
        for component in self.components:
            if getattr(component, "enabled", True):
                try:
                    component.update(self, dt)
                except Exception as error:
                    self.game.console.log(f"Component error: {error}", "ERROR")

    def update_scripts(self, dt):
        if self.game.mode != "PLAY":
            return

        for script in self.scripts:
            if getattr(script, "enabled", True):
                try:
                    if not getattr(script, "started", False):
                        invoke_script_method(script, "start", self)
                        script.started = True

                    invoke_script_method(script, "update", self, dt)

                except Exception as error:
                    self.game.console.log(f"Script error: {error}", "SCRIPT")

    def update_movement(self, dt):
        if self.command == "HOLD":
            self.path = []
            self.state = "HOLD"
            return

        if not self.path:
            if self.command not in ["HOLD", "STOP", "FOLLOW", "GUARD", "GATHER"]:
                self.state = "IDLE"
            return

        self.state = "MOVING"

        target_x, target_y = self.path[0]

        dx = target_x - self.x
        dy = target_y - self.y

        dist = (dx * dx + dy * dy) ** 0.5

        if dist < 0.04:
            self.x = float(target_x)
            self.y = float(target_y)
            self.path.pop(0)
            return

        if dist > 0:
            step = self.speed * dt

            if step > dist:
                step = dist

            smoothing = min(1.0, dt * 12.0)

            target_step_x = self.x + (dx / dist) * step
            target_step_y = self.y + (dy / dist) * step

            self.x += (target_step_x - self.x) * smoothing
            self.y += (target_step_y - self.y) * smoothing

    def update_follow_guard(self):
        if self.command not in ["FOLLOW", "GUARD"]:
            return

        target_id = self.follow_target_id or self.guard_target_id

        if not target_id:
            return

        target = self.game.get_entity_by_id(target_id)

        if not target:
            return

        dx = target.x - self.x
        dy = target.y - self.y
        dist = (dx * dx + dy * dy) ** 0.5

        desired_range = 2.0 if self.command == "GUARD" else 1.2

        if dist > desired_range and not self.path:
            self.game.command_system.move_specific_unit_to(
                self,
                (int(target.x), int(target.y))
            )

    def update_gather(self):
        if self.command != "GATHER":
            return

        worker = self.get_component("Worker")

        if not worker:
            return

        target = self.game.get_entity_by_id(worker.gather_target_id)

        if not target:
            self.command = "IDLE"
            return

        resource = target.get_component("ResourceNode")

        if not resource or resource.is_depleted():
            self.command = "IDLE"
            return

        dx = target.x - self.x
        dy = target.y - self.y
        dist = (dx * dx + dy * dy) ** 0.5

        if dist > 1.2:
            if not self.path:
                self.game.command_system.move_specific_unit_to(
                    self,
                    (int(target.x), int(target.y))
                )
            return

        gathered = resource.gather(resource.gather_rate * 0.016)
        worker.add_resource(resource.resource_type, gathered)

    def apply_separation(self):
        movement = self.get_component("RTSMovement")

        if movement and not movement.separation:
            return

        for other in self.game.units:
            if other is self:
                continue

            if not getattr(other, "enabled", True):
                continue

            dx = self.x - other.x
            dy = self.y - other.y

            dist = (dx * dx + dy * dy) ** 0.5
            min_dist = self.radius + getattr(other, "radius", 0.45)

            if 0 < dist < min_dist:
                push = (min_dist - dist) * 0.03
                self.x += (dx / dist) * push
                self.y += (dy / dist) * push

    # =========================
    # RENDER DATA
    # =========================

    def update_animation(self, dt):
        if self.locked:
            self.animator.play("LOCKED")
        elif self.selected:
            self.animator.play("SELECTED")
        elif self.state == "MOVING":
            self.animator.play("MOVING")
        else:
            self.animator.play("IDLE")

        self.animator.update(dt)

    def get_color(self):
        team = self.get_component("Team")

        if team:
            return tuple(team.color)

        if self.debug_color_flip:
            return (255, 200, 0)

        return self.animator.get_color()

    # =========================
    # SERIALIZATION
    # =========================

    def serialize(self):
        return {
            "type": "Unit",
            "id": self.id,
            "name": self.name,
            "enabled": self.enabled,
            "active": self.enabled,
            "visible": self.visible,
            "locked": self.locked,

            "x": self.x,
            "y": self.y,
            "position": [self.x, self.y],
            "rotation": self.rotation,
            "scale": [self.scale_x, self.scale_y],
            "scale_x": self.scale_x,
            "scale_y": self.scale_y,
            "size": [self.width, self.height],
            "width": self.width,
            "height": self.height,
            "speed": self.speed,
            "radius": self.radius,

            "sprite_name": self.sprite_name,
            "script": self.script,
            "tag": self.tag,
            "layer": self.layer,

            "state": self.state,
            "command": self.command,

            "prefab_source": self.prefab_source,
            "prefab_guid": self.prefab_guid,
            "is_prefab_instance": self.is_prefab_instance,
            "parent_id": self.parent_id,
            "local_x": self.local_x,
            "local_y": self.local_y,

            "patrol_points": self.patrol_points,
            "patrol_index": self.patrol_index,

            "follow_target_id": self.follow_target_id,
            "guard_target_id": self.guard_target_id,
            "attack_move_target": self.attack_move_target,
            "gather_target_id": self.gather_target_id,

            "components": [
                component.serialize()
                for component in self.components
                if hasattr(component, "serialize")
            ],
            "scripts": [
                script.serialize()
                for script in self.scripts
                if hasattr(script, "serialize")
            ]
        }
