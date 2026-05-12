from engine.component import (
    Transform,
    SpriteRenderer,
    RTSMovement,
    Selectable,
    AudioSource,
    Rigidbody2D,
    Animator,
    VisualScript,
    UIElement,
    Collider2D,
    Health,
    Team,
    ResourceNode,
    Worker,
    ADVANCED_COMPONENT_TYPES,
    ADVANCED_COMPONENT_CATEGORIES,
)
from engine.component import component_from_data


class ComponentRegistry:
    """
    Registro central de componentes disponibles.
    FIX:
    - create() siempre devuelve una instancia nueva.
    - evita que dos entidades compartan el mismo componente.
    """

    def __init__(self):
        self.components = {}
        self.metadata = {}
        self.register_defaults()

    def register_defaults(self):
        self.register("Transform", Transform, category="Core", allow_remove=False)
        self.register("SpriteRenderer", SpriteRenderer, category="Rendering")
        self.register("RTSMovement", RTSMovement, category="Gameplay")
        self.register("Selectable", Selectable, category="Editor")
        self.register("AudioSource", AudioSource, category="Audio")
        self.register("Rigidbody2D", Rigidbody2D, category="Physics")
        self.register("Animator", Animator, category="Animation")
        self.register("VisualScript", VisualScript, category="Scripting")
        self.register("UIElement", UIElement, category="UI")
        self.register("Collider2D", Collider2D, category="Physics")
        self.register("Health", Health, category="Gameplay")
        self.register("Team", Team, category="Gameplay")
        self.register("ResourceNode", ResourceNode, category="RTS")
        self.register("Worker", Worker, category="RTS")

        for name, component_class in ADVANCED_COMPONENT_TYPES.items():
            self.register(
                name,
                component_class,
                category=ADVANCED_COMPONENT_CATEGORIES.get(name, "Gameplay"),
            )

    def register(self, name, cls, category="Custom", allow_remove=True):
        self.components[name] = cls
        self.metadata[name] = {
            "category": category,
            "allow_remove": allow_remove,
        }

    def create(self, name, *args, **kwargs):
        cls = self.components.get(name)

        if not cls:
            return None

        return cls(*args, **kwargs)

    def names(self):
        return sorted(self.components.keys())

    def exists(self, name):
        return name in self.components

    def category(self, name):
        return self.metadata.get(name, {}).get("category", "Custom")

    def by_category(self):
        grouped = {}

        for name in self.names():
            grouped.setdefault(self.category(name), []).append(name)

        return grouped

    def clone(self, component):
        if not component or not hasattr(component, "serialize"):
            return None

        return component_from_data(component.serialize())

    def create_from_data(self, data):
        return component_from_data(data)
