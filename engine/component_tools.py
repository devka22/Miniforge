from engine.component import component_from_data
from engine.component_validation import ComponentValidation


class ComponentTools:
    def __init__(self, game):
        self.game = game
        self.clipboard = None
        self.clipboard_type = None
        self.presets = {
            "Platformer Body": [
                {"component_type": "Rigidbody2D", "gravity_scale": 1.2, "drag": 0.02, "freeze_rotation": True},
                {"component_type": "Collider2D", "shape": "rect", "width": 0.85, "height": 0.95},
            ],
            "Trigger Zone": [
                {"component_type": "Collider2D", "shape": "rect", "width": 2.0, "height": 2.0, "is_trigger": True},
            ],
            "Playable Unit": [
                {"component_type": "Selectable"},
                {"component_type": "RTSMovement", "speed": 4.0, "acceleration": 1.5},
                {"component_type": "Collider2D"},
                {"component_type": "Rigidbody2D", "body_type": "kinematic", "use_gravity": False},
            ],
            "Audio Emitter": [
                {"component_type": "AudioSource", "bus": "SFX", "spatial_blend": 0.8, "max_distance": 22.0},
            ],
            "Animated Actor": [
                {"component_type": "Animator", "controller": "Default", "preview": True},
                {"component_type": "SpriteRenderer"},
            ],
            "HUD Button": [
                {"component_type": "UIElement", "element_type": "Button", "text": "Start", "width": 180, "height": 42, "interactable": True},
                {"component_type": "VisualScript", "graph_name": "ButtonGraph"},
            ],
            "TopDown Player": [
                {"component_type": "Stats", "attack": 12, "regen_per_second": 0.25},
                {"component_type": "Health", "max_health": 120},
                {"component_type": "Inventory", "capacity": 32},
                {"component_type": "NavAgent", "speed": 5.0},
                {"component_type": "CameraFollow", "smoothness": 8.0, "zoom": 1.0},
                {"component_type": "Saveable"},
            ],
            "Enemy AI": [
                {"component_type": "Stats", "attack": 8, "defense": 2},
                {"component_type": "Health", "max_health": 60},
                {"component_type": "AIController", "behavior": "attack", "target_tags": ["Player"], "detection_radius": 8},
                {"component_type": "DamageDealer", "damage": 8, "target_tags": ["Player"]},
                {"component_type": "NavAgent", "speed": 3.25},
            ],
            "Pickup Item": [
                {"component_type": "Interaction", "prompt": "Pick up", "single_use": True},
                {"component_type": "Lifetime", "duration": -1},
                {"component_type": "ObjectiveMarker", "label": "Item"},
            ],
            "Quest NPC": [
                {"component_type": "Interaction", "prompt": "Talk"},
                {"component_type": "Dialogue", "speaker": "NPC", "lines": ["Hello.", "Can you help me?"]},
                {"component_type": "QuestLog"},
            ],
            "Combat Projectile": [
                {"component_type": "DamageDealer", "damage": 15, "hit_once": True},
                {"component_type": "Lifetime", "duration": 3},
                {"component_type": "Rigidbody2D", "body_type": "dynamic", "use_gravity": False},
                {"component_type": "Collider2D", "shape": "circle", "radius": 0.2, "is_trigger": True},
            ],
            "Spawner Enemy": [
                {"component_type": "Spawner", "prefab_name": "Enemy", "spawn_interval": 5, "max_alive": 4, "spawn_radius": 3},
                {"component_type": "ObjectiveMarker", "label": "Spawner"},
            ],
            "Checkpoint": [
                {"component_type": "Checkpoint", "checkpoint_id": "checkpoint_01"},
                {"component_type": "Interaction", "prompt": "Activate"},
                {"component_type": "Saveable"},
            ],
            "Platformer Player": [
                {"component_type": "CharacterController2D", "walk_speed": 6.0, "jump_force": 10.0},
                {"component_type": "Rigidbody2D", "gravity_scale": 1.4, "freeze_rotation": True},
                {"component_type": "Collider2D", "shape": "rect", "width": 0.85, "height": 0.95},
                {"component_type": "Health", "max_health": 100},
                {"component_type": "CameraFollow", "smoothness": 10.0},
            ],
            "Interactable Door": [
                {"component_type": "Interaction", "prompt": "Open", "action_graph": "DoorGraph"},
                {"component_type": "StateMachine", "states": ["Closed", "Open"], "current_state": "Closed"},
                {"component_type": "Tween", "property_path": "Transform.scale_y", "from_value": 1.0, "to_value": 0.0, "duration": 0.3},
            ],
        }

    def first_selected(self):
        selected = getattr(self.game, "selected_units", [])
        return selected[0] if selected else None

    def selected_component(self, preferred_type=None):
        entity = self.first_selected()

        if not entity:
            return None

        if preferred_type and hasattr(entity, "get_component"):
            return entity.get_component(preferred_type)

        components = getattr(entity, "components", [])
        return components[0] if components else None

    def copy(self, component_type=None):
        component = self.selected_component(component_type)

        if not component:
            self.log("No hay componente seleccionado para copiar", "WARNING")
            return False

        self.clipboard = component.serialize()
        self.clipboard_type = component.component_type
        self.log(f"Componente copiado: {self.clipboard_type}")
        return True

    def paste(self):
        if not self.clipboard:
            self.log("Portapapeles de componente vacío", "WARNING")
            return 0

        count = 0

        for entity in getattr(self.game, "selected_units", []):
            if not hasattr(entity, "add_component"):
                continue

            component = component_from_data(dict(self.clipboard))

            if not component:
                continue

            existing = entity.get_component(component.component_type)

            if existing:
                entity.remove_component(component.component_type)

            entity.add_component(component)
            ComponentValidation.repair_component(component)
            count += 1

            if hasattr(entity, "sync_from_components"):
                entity.sync_from_components()

        self.after_change(f"Paste Component {self.clipboard_type}")
        self.log(f"Componente pegado en {count} entidad(es)")
        return count

    def reset(self, component_type=None):
        count = 0

        for entity in getattr(self.game, "selected_units", []):
            component = self.selected_on_entity(entity, component_type)

            if not component:
                continue

            fresh = self.game.component_registry.create(component.component_type)

            if not fresh:
                continue

            entity.remove_component(component.component_type)
            entity.add_component(fresh)
            count += 1

        self.after_change("Reset Component")
        self.log(f"Componentes reseteados: {count}")
        return count

    def remove(self, component_type=None):
        count = 0

        for entity in getattr(self.game, "selected_units", []):
            component = self.selected_on_entity(entity, component_type)

            if not component:
                continue

            entity.remove_component(component.component_type)
            count += 1

        self.after_change("Remove Component")
        self.log(f"Componentes eliminados: {count}")
        return count

    def apply_preset(self, preset_name):
        definitions = self.presets.get(preset_name)

        if not definitions:
            self.log(f"No existe preset: {preset_name}", "WARNING")
            return 0

        count = 0

        for entity in getattr(self.game, "selected_units", []):
            for data in definitions:
                component = component_from_data(dict(data))

                if not component:
                    continue

                existing = entity.get_component(component.component_type) if hasattr(entity, "get_component") else None

                if existing:
                    entity.remove_component(component.component_type)

                entity.add_component(component)
                ComponentValidation.repair_component(component)
                count += 1

            if hasattr(entity, "sync_from_components"):
                entity.sync_from_components()

        self.after_change(f"Apply Preset {preset_name}")
        self.log(f"Preset aplicado: {preset_name}")
        return count

    def selected_on_entity(self, entity, component_type=None):
        if component_type and hasattr(entity, "get_component"):
            return entity.get_component(component_type)

        components = getattr(entity, "components", [])
        return components[0] if components else None

    def after_change(self, reason):
        if hasattr(self.game, "history"):
            self.game.history.take_snapshot(reason)

        if hasattr(self.game, "mark_scene_dirty"):
            self.game.mark_scene_dirty(reason)

    def log(self, message, level="ENGINE"):
        if hasattr(self.game, "console"):
            self.game.console.log(message, level)
