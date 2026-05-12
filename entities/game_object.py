from engine.entity_id import generate_entity_id, generate_entity_name
from engine.component import Transform, SpriteRenderer, Selectable, Collider2D, component_from_data
from engine.script import invoke_script_method


class GameObject:
    """
    Entidad genérica tipo Unity.
    Convive con Unit para no romper proyectos RTS existentes.
    """

    def __init__(self, x=0, y=0, game=None, entity_id=None, name=None):
        self.game = game
        self.id = entity_id if entity_id else generate_entity_id()
        self.name = name if name else generate_entity_name("GameObject")
        self.type = "GameObject"

        self.enabled = True
        self.active = True
        self.visible = True
        self.locked = False
        self.selected = False

        self.x = float(x)
        self.y = float(y)
        self.rotation = 0.0
        self.scale_x = 1.0
        self.scale_y = 1.0
        self.width = 1.0
        self.height = 1.0
        self.radius = 0.45
        self.sprite_name = None
        self.sprite_guid = None
        self.script = None
        self.tag = "Untagged"
        self.layer = "Default"
        self.parent_id = None
        self.local_x = 0.0
        self.local_y = 0.0

        self.prefab_source = None
        self.prefab_guid = None
        self.is_prefab_instance = False

        self.components = []
        self.scripts = []

        self.add_component(Transform(self.x, self.y))
        self.add_component(Selectable(True))
        self.add_component(SpriteRenderer(None))
        self.add_component(Collider2D("rect", 1.0, 1.0, self.radius))

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
                    if self.game and hasattr(self.game, "console"):
                        self.game.console.log(f"Script selection error: {error}", "SCRIPT")

    def sync_from_components(self):
        transform = self.get_component("Transform")

        if transform:
            self.x = transform.x
            self.y = transform.y
            self.rotation = getattr(transform, "rotation", self.rotation)
            self.scale_x = getattr(transform, "scale_x", self.scale_x)
            self.scale_y = getattr(transform, "scale_y", self.scale_y)

        sprite = self.get_component("SpriteRenderer")

        if sprite:
            self.sprite_name = sprite.sprite_name

        collider = self.get_component("Collider2D")

        if collider:
            self.radius = collider.radius
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

        sprite = self.get_component("SpriteRenderer")

        if sprite:
            sprite.sprite_name = self.sprite_name

        collider = self.get_component("Collider2D")

        if collider:
            collider.radius = self.radius
            collider.width = self.width
            collider.height = self.height

    def update(self, dt=0.016):
        if not self.enabled:
            return

        self.sync_from_components()

        for component in self.components:
            if not getattr(component, "enabled", True):
                continue

            try:
                component.update(self, dt)
            except Exception as error:
                if self.game and hasattr(self.game, "console"):
                    self.game.console.log(f"Component error: {error}", "ERROR")

        if self.game and getattr(self.game, "mode", "EDITOR") == "PLAY":
            for script in self.scripts:
                if not getattr(script, "enabled", True):
                    continue

                try:
                    if not getattr(script, "started", False):
                        invoke_script_method(script, "start", self)
                        script.started = True

                    invoke_script_method(script, "update", self, dt)
                except Exception as error:
                    if self.game and hasattr(self.game, "console"):
                        self.game.console.log(f"Script error: {error}", "SCRIPT")

        self.sync_to_components()

    def serialize(self):
        self.sync_from_components()

        return {
            "type": "GameObject",
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
            "radius": self.radius,
            "sprite_name": self.sprite_name,
            "sprite_guid": self.sprite_guid,
            "script": self.script,
            "tag": self.tag,
            "layer": self.layer,
            "parent_id": self.parent_id,
            "local_x": self.local_x,
            "local_y": self.local_y,
            "prefab_source": self.prefab_source,
            "prefab_guid": self.prefab_guid,
            "is_prefab_instance": self.is_prefab_instance,
            "components": [
                component.serialize()
                for component in self.components
            ],
            "scripts": [
                script.serialize()
                for script in self.scripts
                if hasattr(script, "serialize")
            ],
        }


def game_object_from_data(game, data, preserve_id=False):
    entity_id = data.get("id") if preserve_id else None

    obj = GameObject(
        data.get("x", 0),
        data.get("y", 0),
        game=game,
        entity_id=entity_id,
        name=data.get("name"),
    )

    position = data.get("position")

    if isinstance(position, (list, tuple)) and len(position) >= 2:
        obj.x = float(position[0])
        obj.y = float(position[1])

    obj.enabled = data.get("enabled", data.get("active", True))
    obj.active = obj.enabled
    obj.visible = data.get("visible", True)
    obj.locked = data.get("locked", False)
    obj.rotation = data.get("rotation", 0.0)

    scale = data.get("scale", [data.get("scale_x", 1.0), data.get("scale_y", 1.0)])
    size = data.get("size", [data.get("width", 1.0), data.get("height", 1.0)])

    if isinstance(scale, (list, tuple)) and len(scale) >= 2:
        obj.scale_x = float(scale[0])
        obj.scale_y = float(scale[1])

    if isinstance(size, (list, tuple)) and len(size) >= 2:
        obj.width = float(size[0])
        obj.height = float(size[1])

    obj.radius = data.get("radius", 0.45)
    obj.sprite_name = data.get("sprite_name")
    obj.sprite_guid = data.get("sprite_guid")
    obj.script = data.get("script")
    obj.tag = data.get("tag", "Untagged")
    obj.layer = data.get("layer", "Default")
    obj.parent_id = data.get("parent_id")
    obj.local_x = data.get("local_x", 0.0)
    obj.local_y = data.get("local_y", 0.0)
    obj.prefab_source = data.get("prefab_source")
    obj.prefab_guid = data.get("prefab_guid")
    obj.is_prefab_instance = data.get("is_prefab_instance", False)

    obj.components.clear()

    for component_data in data.get("components", []):
        component = component_from_data(component_data)

        if component:
            obj.add_component(component)

    for script_data in data.get("scripts", []):
        script_name = script_data.get("script") or script_data.get("script_name")

        if not script_name:
            continue

        script = game.script_manager.create(script_name)

        if script:
            if hasattr(script, "deserialize"):
                script.deserialize(script_data)

            obj.add_script(script)

    obj.sync_to_components()
    return obj
