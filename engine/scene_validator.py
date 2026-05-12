import os
from engine.component_validation import ComponentValidation


class SceneValidator:
    """
    Scene Validation 2.

    Revisa:
    - IDs duplicados
    - entidades fuera del mapa
    - sprites faltantes
    - scripts faltantes
    - prefabs rotos
    - componentes inválidos
    - tiles inválidos
    - tags/layers inválidos
    """

    def __init__(self, game):
        self.game = game
        self.warnings = []
        self.errors = []

    def validate(self):
        self.warnings = []
        self.errors = []

        self.check_duplicate_ids()
        self.check_entities_inside_map()
        self.check_missing_sprites()
        self.check_missing_scripts()
        self.check_broken_prefabs()
        self.check_invalid_components()
        self.check_invalid_tiles()
        self.check_tilemap_layers()
        self.check_tags_layers()
        self.check_control_groups()
        self.check_hierarchy_cycles()
        self.check_ui_elements()
        self.check_animation_controllers()
        self.check_gameplay_components()

        self.print_results()

        return len(self.errors) == 0

    def check_duplicate_ids(self):
        ids = set()

        for unit in self.game.units:
            entity_id = getattr(unit, "id", None)

            if not entity_id:
                self.errors.append(f"Entidad sin ID: {getattr(unit, 'name', 'Unknown')}")
                continue

            if entity_id in ids:
                self.errors.append(f"ID duplicado: {entity_id}")
            else:
                ids.add(entity_id)

    def check_entities_inside_map(self):
        for unit in self.game.units:
            x = int(unit.x)
            y = int(unit.y)

            if not self.game.grid.is_inside(x, y):
                self.warnings.append(f"Entidad fuera del mapa: {unit.name} ({x}, {y})")

    def check_missing_sprites(self):
        for unit in self.game.units:
            if not getattr(unit, "visible", True):
                continue

            sprite_name = getattr(unit, "sprite_name", None)

            if not sprite_name:
                continue

            sprite = self.game.resources.get_image(sprite_name)

            if sprite is None:
                self.warnings.append(f"Sprite faltante en {unit.name}: {sprite_name}")

    def check_missing_scripts(self):
        for unit in self.game.units:
            for script in getattr(unit, "scripts", []):
                script_name = getattr(script, "script_name", None)

                if not script_name:
                    self.warnings.append(f"Script sin nombre en {unit.name}")
                    continue

                filename = script_name

                if not filename.endswith(".py"):
                    filename += ".py"

                possible_path = os.path.join("scripts", filename)

                if not os.path.exists(possible_path):
                    self.warnings.append(f"Script faltante en {unit.name}: {filename}")

    def check_broken_prefabs(self):
        for unit in self.game.units:
            if not getattr(unit, "is_prefab_instance", False):
                continue

            prefab_source = getattr(unit, "prefab_source", None)

            if not prefab_source:
                self.warnings.append(f"Prefab instance sin source: {unit.name}")
                continue

            if not os.path.exists(prefab_source):
                self.warnings.append(f"Prefab source roto en {unit.name}: {prefab_source}")

    def check_invalid_components(self):
        for unit in self.game.units:
            seen = set()

            for component in getattr(unit, "components", []):
                component_type = getattr(component, "component_type", None)

                if not component_type:
                    self.errors.append(f"Componente inválido en {unit.name}")
                    continue

                if component_type in seen:
                    self.warnings.append(f"Componente duplicado en {unit.name}: {component_type}")

                seen.add(component_type)
                warnings, errors = ComponentValidation.validate_component(component)

                for warning in warnings:
                    self.warnings.append(f"{unit.name}: {warning}")

                for error in errors:
                    self.errors.append(f"{unit.name}: {error}")

    def check_invalid_tiles(self):
        valid_tiles = {0, 1, 2, 3, 4}

        for y, row in enumerate(self.game.grid.tiles):
            for x, tile in enumerate(row):
                if tile not in valid_tiles:
                    self.warnings.append(f"Tile inválido en ({x},{y}): {tile}")

    def check_tilemap_layers(self):
        tilemap = getattr(self.game, "tilemap_layers", None)

        if not tilemap:
            return

        if tilemap.width != self.game.grid.width or tilemap.height != self.game.grid.height:
            self.warnings.append("Tilemap layers no coincide con tamaño del grid")

        for layer in tilemap.layers:
            if layer.width != tilemap.width or layer.height != tilemap.height:
                self.warnings.append(f"Layer {layer.name} tiene tamaño inconsistente")

            for y, row in enumerate(layer.tiles):
                for x, tile in enumerate(row):
                    if tile < layer.default_tile:
                        self.warnings.append(f"Tile inválido en layer {layer.name} ({x},{y}): {tile}")
                        return

    def check_tags_layers(self):
        tags = getattr(self.game, "tags", ["Untagged"])
        layers = getattr(self.game, "layers", ["Default"])

        for unit in self.game.units:
            if getattr(unit, "tag", "Untagged") not in tags:
                self.warnings.append(f"Tag no registrado en {unit.name}: {unit.tag}")

            if getattr(unit, "layer", "Default") not in layers:
                self.warnings.append(f"Layer no registrada en {unit.name}: {unit.layer}")

    def check_control_groups(self):
        ids = {unit.id for unit in self.game.units}

        for group, group_ids in getattr(self.game, "control_groups", {}).items():
            for entity_id in group_ids:
                if entity_id not in ids:
                    self.warnings.append(f"Control group {group} contiene ID inexistente: {entity_id}")

    def check_hierarchy_cycles(self):
        by_id = {unit.id: unit for unit in self.game.units}

        for unit in self.game.units:
            visited = set()
            current = unit

            while getattr(current, "parent_id", None):
                parent_id = current.parent_id

                if parent_id in visited:
                    self.errors.append(f"Ciclo de jerarquía detectado en {unit.name}")
                    break

                visited.add(parent_id)
                current = by_id.get(parent_id)

                if current is None:
                    self.warnings.append(f"Parent inexistente en {unit.name}: {parent_id}")
                    break

    def check_ui_elements(self):
        screen_w, screen_h = self.game.screen.get_size() if hasattr(self.game, "screen") else (1100, 740)

        for unit in self.game.units:
            ui = unit.get_component("UIElement") if hasattr(unit, "get_component") else None

            if not ui:
                continue

            if ui.width <= 0 or ui.height <= 0:
                self.errors.append(f"UIElement con tamaño inválido en {unit.name}")

            if abs(ui.x) > screen_w * 4 or abs(ui.y) > screen_h * 4:
                self.warnings.append(f"UIElement muy lejos del canvas en {unit.name}")

    def check_animation_controllers(self):
        library = getattr(self.game, "animation_graphs", None)

        if not library:
            return

        names = set(library.names())

        for unit in self.game.units:
            animator = unit.get_component("Animator") if hasattr(unit, "get_component") else None

            if animator and animator.controller not in names:
                self.warnings.append(f"Animator controller faltante en {unit.name}: {animator.controller}")

    def check_gameplay_components(self):
        ids = {unit.id for unit in self.game.units}
        save_keys = {}

        for unit in self.game.units:
            if not hasattr(unit, "get_component"):
                continue

            saveable = unit.get_component("Saveable")

            if saveable:
                key = getattr(saveable, "save_key", "") or getattr(unit, "id", "")

                if key in save_keys:
                    self.warnings.append(f"Saveable key duplicada: {key} en {unit.name}")
                else:
                    save_keys[key] = unit

            ai = unit.get_component("AIController")

            if ai and getattr(ai, "target_id", None) and ai.target_id not in ids:
                self.warnings.append(f"AI target inexistente en {unit.name}: {ai.target_id}")

            nav = unit.get_component("NavAgent")

            if nav and getattr(nav, "has_destination", False):
                x = int(getattr(nav, "destination_x", 0))
                y = int(getattr(nav, "destination_y", 0))

                if not self.game.grid.is_inside(x, y):
                    self.warnings.append(f"NavAgent destino fuera del mapa en {unit.name}: ({x}, {y})")

            inventory = unit.get_component("Inventory")

            if inventory and len(getattr(inventory, "items", [])) > int(getattr(inventory, "capacity", 0)):
                self.warnings.append(f"Inventory sobre capacidad en {unit.name}")

            spawner = unit.get_component("Spawner")

            if spawner and int(getattr(spawner, "max_alive", 0)) == 0:
                self.warnings.append(f"Spawner sin capacidad en {unit.name}")

            tween = unit.get_component("Tween")

            if tween and getattr(tween, "active", False) and not getattr(tween, "property_path", ""):
                self.errors.append(f"Tween activo sin property_path en {unit.name}")

    def print_results(self):
        if not self.warnings and not self.errors:
            self.game.console.log("Scene validation: OK", "SCENE")
            return

        for warning in self.warnings[:12]:
            self.game.console.log(warning, "WARNING")

        if len(self.warnings) > 12:
            self.game.console.log(f"... {len(self.warnings) - 12} warnings más", "WARNING")

        for error in self.errors:
            self.game.console.log(error, "ERROR")
