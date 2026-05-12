import json
import math
import os

from entities.game_object import GameObject
from entities.unit import Unit
from engine.component import component_from_data
from engine.prefab_manager import PrefabManager


class GameAPI:
    """
    API estable para scripts de proyecto.
    Evita que los juegos dependan de detalles internos del editor.
    """

    def __init__(self, game):
        self.game = game

    def find(self, name):
        for entity in self.game.units:
            if getattr(entity, "name", None) == name:
                return entity

        return None

    def entities(self, enabled_only=False):
        entities = list(getattr(self.game.world, "entities", self.game.units))

        if enabled_only:
            return [entity for entity in entities if getattr(entity, "enabled", True)]

        return entities

    def find_by_id(self, entity_id):
        return self.game.get_entity_by_id(entity_id)

    def find_with_tag(self, tag):
        return [
            entity for entity in self.game.units
            if getattr(entity, "tag", None) == tag
        ]

    def find_with_component(self, component_type):
        return [
            entity for entity in self.entities()
            if hasattr(entity, "get_component") and entity.get_component(component_type)
        ]

    def get_component(self, entity, component_type):
        if not entity or not hasattr(entity, "get_component"):
            return None

        return entity.get_component(component_type)

    def add_component(self, entity, component_name, data=None):
        if not entity or not hasattr(entity, "add_component"):
            return None

        component = None

        if data:
            payload = dict(data)
            payload.setdefault("component_type", component_name)
            component = component_from_data(payload)

        if not component and hasattr(self.game, "component_registry"):
            component = self.game.component_registry.create(component_name)

        if not component:
            component = component_from_data({"component_type": component_name})

        if not component:
            return None

        added_component = entity.add_component(component)

        if hasattr(entity, "sync_from_components"):
            entity.sync_from_components()

        if hasattr(entity, "sync_to_components"):
            entity.sync_to_components()

        if hasattr(self.game, "mark_scene_dirty"):
            self.game.mark_scene_dirty(f"API Add Component {component_name}")

        return added_component or component

    def remove_component(self, entity, component_type):
        if not entity or not hasattr(entity, "remove_component"):
            return False

        entity.remove_component(component_type)
        return True

    def create_game_object(self, name="GameObject", x=0, y=0):
        obj = GameObject(x, y, self.game, name=name)
        self.game.units.append(obj)
        self.game.world.entities = self.game.units
        return obj

    def create_unit(self, name="Unit", x=0, y=0):
        unit = Unit(x, y, self.game, name=name)
        self.game.units.append(unit)
        self.game.world.entities = self.game.units
        return unit

    def instantiate(self, prefab_path_or_name, x=0, y=0):
        asset = None

        if hasattr(self.game, "file_browser"):
            asset = self.game.file_browser.find_asset_by_name(prefab_path_or_name)

        path = prefab_path_or_name

        if asset:
            path = asset.get("path")

        return PrefabManager.instantiate_prefab(self.game, path, x, y)

    def destroy(self, entity):
        if entity in self.game.units:
            self.game.units.remove(entity)
            self.game.world.entities = self.game.units
            return True

        return False

    def destroy_by_id(self, entity_id):
        entity = self.find_by_id(entity_id)
        return self.destroy(entity) if entity else False

    def set_position(self, entity, x, y):
        if not entity:
            return False

        entity.x = float(x)
        entity.y = float(y)

        if hasattr(entity, "sync_to_components"):
            entity.sync_to_components()

        return True

    def translate(self, entity, dx=0, dy=0):
        if not entity:
            return False

        return self.set_position(
            entity,
            getattr(entity, "x", 0.0) + float(dx),
            getattr(entity, "y", 0.0) + float(dy),
        )

    def move_to(self, entity, x, y):
        if not entity:
            return False

        nav = self.get_component(entity, "NavAgent")

        if nav:
            nav.set_destination(x, y)
            return True

        if hasattr(self.game, "command_system") and hasattr(entity, "path"):
            return self.game.command_system.move_specific_unit_to(entity, (int(x), int(y)))

        return self.set_position(entity, x, y)

    def query_radius(self, x, y, radius, tag=None, component_type=None):
        found = []
        radius = float(radius)

        for entity in self.entities(enabled_only=True):
            if tag is not None and getattr(entity, "tag", None) != tag:
                continue

            if component_type and not self.get_component(entity, component_type):
                continue

            dx = float(getattr(entity, "x", 0.0)) - float(x)
            dy = float(getattr(entity, "y", 0.0)) - float(y)

            if math.hypot(dx, dy) <= radius:
                found.append(entity)

        return found

    def nearest(self, x, y, tag=None, component_type=None, max_distance=None):
        best = None
        best_distance = float(max_distance) if max_distance is not None else float("inf")

        for entity in self.query_radius(x, y, best_distance, tag, component_type):
            dx = float(getattr(entity, "x", 0.0)) - float(x)
            dy = float(getattr(entity, "y", 0.0)) - float(y)
            distance = math.hypot(dx, dy)

            if distance < best_distance:
                best = entity
                best_distance = distance

        return best

    def damage(self, entity, amount):
        health = self.get_component(entity, "Health")

        if not health:
            return False

        health.take_damage(amount)
        return True

    def heal(self, entity, amount):
        health = self.get_component(entity, "Health")

        if not health:
            return False

        health.heal(amount)
        return True

    def health(self, entity):
        health = self.get_component(entity, "Health")
        return getattr(health, "health", None) if health else None

    def add_item(self, entity, item_id, quantity=1, metadata=None):
        inventory = self.get_component(entity, "Inventory")

        if not inventory:
            inventory = self.add_component(entity, "Inventory")

        return inventory.add_item(item_id, quantity, metadata) if inventory else 0

    def remove_item(self, entity, item_id, quantity=1):
        inventory = self.get_component(entity, "Inventory")
        return inventory.remove_item(item_id, quantity) if inventory else 0

    def item_count(self, entity, item_id):
        inventory = self.get_component(entity, "Inventory")
        return inventory.count_item(item_id) if inventory else 0

    def add_resource(self, entity, resource_type, amount):
        wallet = self.get_component(entity, "EconomyWallet")

        if not wallet:
            wallet = self.add_component(entity, "EconomyWallet")

        return wallet.add(resource_type, amount) if wallet else None

    def spend_resource(self, entity, resource_type, amount):
        wallet = self.get_component(entity, "EconomyWallet")
        return wallet.spend(resource_type, amount) if wallet else False

    def set_blackboard(self, entity, key, value):
        blackboard = self.get_component(entity, "Blackboard")

        if not blackboard:
            blackboard = self.add_component(entity, "Blackboard")

        if not blackboard:
            return False

        blackboard.set(key, value)
        return True

    def get_blackboard(self, entity, key, default=None):
        blackboard = self.get_component(entity, "Blackboard")
        return blackboard.get(key, default) if blackboard else default

    def start_cooldown(self, entity, name, duration):
        cooldown = self.get_component(entity, "Cooldown")

        if not cooldown:
            cooldown = self.add_component(entity, "Cooldown")

        if not cooldown:
            return False

        cooldown.start(name, duration)
        return True

    def cooldown_ready(self, entity, name):
        cooldown = self.get_component(entity, "Cooldown")
        return True if not cooldown else cooldown.ready(name)

    def add_status_effect(self, entity, name, duration, stacks=1, data=None):
        status = self.get_component(entity, "StatusEffects")

        if not status:
            status = self.add_component(entity, "StatusEffects")

        if not status:
            return False

        status.add_effect(name, duration, stacks, data)
        return True

    def tween(self, entity, property_path, to_value, duration=1.0, easing="smooth"):
        tween = self.get_component(entity, "Tween")

        if not tween:
            tween = self.add_component(entity, "Tween")

        if not tween:
            return False

        tween.property_path = property_path
        tween.from_value = float(self.read_property_path(entity, property_path, 0.0))
        tween.to_value = float(to_value)
        tween.duration = max(0.0, float(duration))
        tween.elapsed = 0.0
        tween.easing = easing
        tween.active = True
        return True

    def read_property_path(self, entity, property_path, default=None):
        if "." in property_path:
            component_type, attr = property_path.split(".", 1)
            component = self.get_component(entity, component_type)
            return getattr(component, attr, default) if component else default

        return getattr(entity, property_path, default)

    def add_quest(self, entity, quest_id, title, objectives=None):
        quest_log = self.get_component(entity, "QuestLog")

        if not quest_log:
            quest_log = self.add_component(entity, "QuestLog")

        return quest_log.add_quest(quest_id, title, objectives) if quest_log else False

    def complete_quest(self, entity, quest_id):
        quest_log = self.get_component(entity, "QuestLog")
        return quest_log.complete_quest(quest_id) if quest_log else False

    def on(self, event_name, callback):
        if hasattr(self.game, "event_bus"):
            self.game.event_bus.subscribe(event_name, callback)
            return True
        return False

    def emit(self, event_name, data=None):
        if hasattr(self.game, "event_bus"):
            self.game.event_bus.emit(event_name, data)
            return True
        return False

    def load_scene(self, scene_name):
        if not scene_name.endswith(".scene"):
            scene_name += ".scene"

        self.game.scene_manager.current_scene = scene_name
        return self.game.scene_manager.load_current_scene()

    def save_game_state(self, filename="savegame.json"):
        path = self.game.project_join("saves", filename) if hasattr(self.game, "project_join") else filename
        folder = os.path.dirname(path)

        if folder:
            os.makedirs(folder, exist_ok=True)

        data = {
            "entities": [
                entity.serialize()
                for entity in self.entities()
                if self.get_component(entity, "Saveable")
            ]
        }

        with open(path, "w", encoding="utf-8") as file:
            json.dump(data, file, indent=4)

        return path

    def load_game_state(self, filename="savegame.json"):
        path = self.game.project_join("saves", filename) if hasattr(self.game, "project_join") else filename

        if not os.path.exists(path):
            return False

        with open(path, "r", encoding="utf-8") as file:
            data = json.load(file)

        by_key = {}

        for entity in self.entities():
            saveable = self.get_component(entity, "Saveable")
            key = getattr(saveable, "save_key", None) if saveable else None

            if key:
                by_key[key] = entity

        for entity_data in data.get("entities", []):
            key = None

            for component in entity_data.get("components", []):
                if component.get("component_type") == "Saveable":
                    key = component.get("save_key")
                    break

            entity = by_key.get(key) if key else None

            if entity:
                entity.x = entity_data.get("x", entity.x)
                entity.y = entity_data.get("y", entity.y)

                if hasattr(entity, "sync_to_components"):
                    entity.sync_to_components()

        return True

    def get_key(self, key_name):
        import pygame

        key = getattr(pygame, f"K_{str(key_name).lower()}", None)

        if key is None:
            return False

        return pygame.key.get_pressed()[key]

    def log(self, message, level="SCRIPT"):
        self.game.console.log(str(message), level)
