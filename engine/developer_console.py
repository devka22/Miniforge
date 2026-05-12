import os
import datetime

from engine.version import ENGINE_VERSION, version_label


class DeveloperConsole:
    """
    Console Pro 0.6.0.
    - Guarda logs en logs globales y logs del proyecto.
    - Comandos para crear archivos del proyecto.
    - Comandos para browser.
    """

    def __init__(self):
        self.visible = True
        self.logs = []
        self.max_logs = 6
        self.history_limit = 500
        self.input_active = False
        self.command_buffer = ""
        self.game = None
        self.compact = True
        self.filter_level = "ALL"

        self.last_error = None
        self.last_warning = None

        self.log_file = "logs/engine.log"
        self.error_log_file = "logs/error.log"
        os.makedirs("logs", exist_ok=True)

    def attach_game(self, game):
        self.game = game

        try:
            project_logs = game.project_logs_path()
            os.makedirs(project_logs, exist_ok=True)
            self.log_file = os.path.join(project_logs, "engine.log")
            self.error_log_file = os.path.join(project_logs, "error.log")
        except Exception:
            pass

    def toggle(self):
        self.visible = not self.visible
        self.input_active = False

    def cycle_filter(self):
        filters = ["ALL", "ERROR", "WARNING", "SCRIPT", "ENGINE", "ASSET", "SCENE"]
        index = filters.index(self.filter_level) if self.filter_level in filters else 0
        self.filter_level = filters[(index + 1) % len(filters)]
        self.log(f"Console filter: {self.filter_level}", "ENGINE")

    def visible_logs(self):
        if self.filter_level == "ALL":
            return self.logs[-self.max_logs:]

        return [
            entry for entry in self.logs
            if isinstance(entry, dict) and entry.get("level") == self.filter_level
        ][-self.max_logs:]

    def toggle_input(self):
        self.visible = True
        self.input_active = not self.input_active
        self.command_buffer = ""

        if self.input_active:
            self.log("Console input activo. Escribe help.", "ENGINE")

    def write_to_file(self, entry):
        try:
            timestamp = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")

            os.makedirs(os.path.dirname(self.log_file), exist_ok=True)

            with open(self.log_file, "a", encoding="utf-8") as file:
                file.write(f"[{timestamp}] [{entry['level']}] {entry['message']}\n")

            if entry["level"] == "ERROR":
                with open(self.error_log_file, "a", encoding="utf-8") as file:
                    file.write(f"[{timestamp}] [{entry['level']}] {entry['message']}\n")

        except Exception:
            pass

    def log(self, message, level="INFO"):
        level = level.upper()

        entry = {
            "level": level,
            "message": str(message),
        }

        self.logs.append(entry)

        if len(self.logs) > self.history_limit:
            self.logs.pop(0)

        if level == "ERROR":
            self.last_error = str(message)

        if level == "WARNING":
            self.last_warning = str(message)

        self.write_to_file(entry)
        print(f"[{entry['level']}] {entry['message']}")

    def clear(self):
        self.logs.clear()
        self.last_error = None
        self.last_warning = None

    def submit_command(self):
        command = self.command_buffer.strip()
        self.command_buffer = ""

        if not command:
            return

        self.log(f"> {command}", "ENGINE")
        self.execute(command)

    def execute(self, command):
        if not self.game:
            self.log("Console no conectada al Game", "ERROR")
            return

        parts = command.split()
        base = parts[0].lower()

        try:
            handlers = {
                "help": self.print_help,
                "clear": self.clear,
                "clear_logs": self.clear,
                "save": self.game.save_scene,
                "load": self.game.load_scene,
                "new_scene": self.game.new_scene,
                "reload": self.game.refresh_project,
                "recover": self.game.recover_autosave,
                "autosave": self.game.autosave_now,
                "validate": self.game.validate_scene,
                "manifest": self.game.build_manifest,
                "view": self.game.toggle_view_mode,
                "center": self.game.center_camera_on_selection,
                "version": self.print_version,
            }

            if base in handlers:
                handlers[base]()

            elif base == "create":
                self.create_command(parts)

            elif base == "browser":
                self.browser_command(parts)

            elif base == "scene":
                self.scene_command(parts)

            elif base == "play":
                if self.game.mode == "EDITOR":
                    self.game.toggle_mode()
                else:
                    self.log("Ya estás en PLAY MODE", "WARNING")

            elif base == "editor":
                if self.game.mode == "PLAY":
                    self.game.toggle_mode()
                else:
                    self.log("Ya estás en EDITOR MODE", "WARNING")

            elif base == "tool":
                self.tool_command(parts)

            elif base == "spawn":
                self.spawn_command(parts)

            elif base == "delete":
                self.game.delete_selected()

            elif base == "duplicate":
                self.game.duplicate_selected()

            elif base == "rename":
                self.rename_command(parts)

            elif base == "set":
                self.set_command(parts)

            elif base == "brush":
                self.brush_command(parts)

            elif base == "select":
                self.select_command(parts)

            elif base == "asset":
                self.asset_command(parts)

            elif base == "ui":
                self.ui_command(parts)

            elif base == "visual":
                self.visual_command(parts)

            elif base == "plugin":
                self.plugin_command(parts)

            elif base == "example":
                self.example_command(parts)

            elif base == "prefab":
                self.prefab_command(parts)

            elif base == "layout":
                self.layout_command(parts)

            elif base == "tag":
                self.tag_command(parts)

            elif base == "layer":
                self.layer_command(parts)

            elif base == "show":
                self.show_command(parts)

            elif base == "rts":
                self.rts_command(parts)

            elif base == "component":
                self.component_command(parts)

            elif base == "groups":
                self.groups_command()

            elif base == "find":
                self.find_command(parts)

            elif base == "build":
                self.build_command(parts)

            else:
                self.log(f"Comando desconocido: {base}", "WARNING")

        except Exception as error:
            self.log(f"Error ejecutando comando: {error}", "ERROR")

    # =========================
    # CREATE FILES
    # =========================

    def create_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: create script/component/system/scene/prefab/json/txt/folder nombre", "WARNING")
            return

        kind = parts[1].lower()
        name = parts[2] if len(parts) >= 3 else None

        fb = self.game.file_browser

        if kind == "script":
            fb.create_script(name or "NewScript")

        elif kind == "component":
            fb.create_component(name or "NewComponent")

        elif kind == "system":
            fb.create_system(name or "NewSystem")

        elif kind == "scene":
            fb.create_scene(name or "NewScene")

        elif kind == "prefab":
            fb.create_prefab(name or "NewPrefab")

        elif kind == "json":
            fb.create_json(name or "NewData")

        elif kind == "txt":
            fb.create_txt(name or "NewText")

        elif kind == "folder":
            fb.create_folder(name or "NewFolder")

        elif kind in ["sprites", "audio", "data", "components", "systems", "scenes", "plugins"]:
            fb.create_special_folder(kind)

        else:
            self.log("Tipo create inválido", "WARNING")
            return

        self.game.refresh_project()

    # =========================
    # BROWSER COMMANDS
    # =========================

    def browser_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: browser open/duplicate/delete/rename/rename_folder/tree/refresh", "WARNING")
            return

        action = parts[1].lower()
        fb = self.game.file_browser

        if action == "open":
            fb.open_selected()

        elif action == "duplicate":
            fb.duplicate_selected_asset()

        elif action == "delete":
            fb.delete_selected_asset()

        elif action == "rename" and len(parts) >= 3:
            fb.rename_selected_asset(parts[2])

        elif action == "rename_folder" and len(parts) >= 3:
            fb.rename_selected_folder(parts[2])

        elif action == "tree":
            fb.toggle_tree_view()

        elif action == "refresh":
            self.game.refresh_project()

        else:
            self.log("Comando browser inválido", "WARNING")

    # =========================
    # SCENE
    # =========================

    def scene_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: scene save/load/rename/duplicate/delete/saveas", "WARNING")
            return

        action = parts[1].lower()

        if action == "save":
            self.game.save_scene()
        elif action == "load":
            self.game.load_scene()
        elif action == "rename" and len(parts) >= 3:
            self.game.scene_tools.rename_current_scene(parts[2])
        elif action == "duplicate":
            self.game.scene_tools.duplicate_current_scene()
        elif action == "delete":
            self.game.scene_tools.delete_current_scene()
        elif action == "saveas" and len(parts) >= 3:
            self.game.scene_tools.save_scene_as(parts[2])
        else:
            self.log("Comando scene inválido", "WARNING")

    def tool_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: tool select/move/entity/tile/obstacle/erase", "WARNING")
            return

        tool_name = parts[1].capitalize()
        valid = ["Select", "Move", "Entity", "Tile", "Obstacle", "Erase"]

        if tool_name in valid:
            self.game.active_tool = tool_name
            self.log(f"Herramienta: {tool_name}", "ENGINE")
        else:
            self.log("Herramienta inválida", "WARNING")

    def spawn_command(self, parts):
        if len(parts) >= 2 and parts[1].lower() == "player":
            entity = self.game.create_game_object()
            entity.name = "Player"
            entity.tag = "Player"
            entity.type = "Player"
            self.log("Player creado", "ENGINE")
            return

        if len(parts) >= 3:
            self.game.spawn_unit_at_grid(int(parts[1]), int(parts[2]))
        else:
            self.game.spawn_unit()

    def rename_command(self, parts):
        if len(parts) >= 3 and parts[1].lower() == "selected":
            self.game.rename_selected_entity(" ".join(parts[2:]))
        else:
            self.log("Uso: rename selected NuevoNombre", "WARNING")

    def set_command(self, parts):
        if len(parts) < 4:
            self.log("Uso: set selected campo valor", "WARNING")
            return

        if parts[1].lower() != "selected":
            self.log("Solo se soporta: set selected campo valor", "WARNING")
            return

        if not self.game.selected_units:
            self.log("No hay entidad seleccionada", "WARNING")
            return

        unit = self.game.selected_units[0]
        field = parts[2]
        value = " ".join(parts[3:])

        if field in ["x", "y", "speed", "radius"]:
            setattr(unit, field, float(value))
        elif field in ["name", "tag", "layer", "sprite_name", "command"]:
            setattr(unit, field, value)
        elif field in ["enabled", "visible", "locked"]:
            setattr(unit, field, value.lower() in ["true", "1", "yes", "si", "sí"])
        else:
            self.log("Campo no soportado", "WARNING")
            return

        unit.sync_to_components()
        self.game.history.take_snapshot(f"Console set {field}")
        self.log(f"{field} cambiado a {value}", "ENGINE")

    def select_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: select nombre_o_id", "WARNING")
            return

        query = " ".join(parts[1:]).lower()

        if hasattr(self.game, "selection_manager"):
            self.game.selection_manager.clear()
        else:
            self.game.clear_selection()

        found = False

        for unit in self.game.units:
            name = getattr(unit, "name", "").lower()
            entity_id = getattr(unit, "id", "").lower()

            if query in name or query == entity_id:
                if hasattr(self.game, "selection_manager"):
                    self.game.selection_manager.add(unit)
                else:
                    self.game.add_to_selection(unit)

                found = True

        if found:
            self.log(f"Entidad seleccionada: {query}", "ENGINE")
        else:
            self.log("No se encontró entidad", "WARNING")

    def find_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: find texto", "WARNING")
            return

        query = " ".join(parts[1:])

        if hasattr(self.game, "scene_hierarchy"):
            self.game.scene_hierarchy.set_search(query)
            self.log(f"Hierarchy search: {query}", "EDITOR")

    def brush_command(self, parts):
        if len(parts) >= 3 and parts[1].lower() == "size":
            size = int(parts[2])
            self.game.set_brush_size(size)
            return

        if len(parts) >= 2 and parts[1].isdigit():
            self.game.tile_brush = max(0, min(4, int(parts[1])))
            self.log(f"Brush: {self.game.tile_brush_name()}", "ENGINE")
            return

        self.game.next_tile_brush()

    def asset_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: asset refresh/delete/rename/folder/open/tree/duplicate/deps/import", "WARNING")
            return

        action = parts[1].lower()
        fb = self.game.file_browser

        if action == "refresh":
            self.game.refresh_project()
        elif action == "delete":
            fb.delete_selected_asset()
        elif action == "rename" and len(parts) >= 3:
            fb.rename_selected_asset(parts[2])
        elif action == "folder":
            fb.create_folder()
        elif action == "open":
            fb.open_selected()
        elif action == "tree":
            fb.toggle_tree_view()
        elif action == "duplicate":
            fb.duplicate_selected_asset()
        elif action == "deps":
            self.game.print_selected_asset_dependencies()
        elif action == "import":
            self.game.cycle_selected_asset_import_setting()
        elif action == "graph":
            self.game.rebuild_asset_dependency_graph()
        else:
            self.log("Comando asset inválido", "WARNING")

    def ui_command(self, parts):
        kind = parts[1].lower() if len(parts) > 1 else "label"
        text = " ".join(parts[2:]) if len(parts) > 2 else None

        if kind == "label":
            self.game.create_ui_label(text or "Label")
        elif kind == "button":
            self.game.create_ui_button(text or "Button")
        elif kind in ("bar", "progress", "progressbar"):
            self.game.create_ui_progress_bar(text or "Progress")
        elif kind == "example":
            self.game.create_example_ui_scene()
        else:
            self.log("Uso: ui label/button/progress/example", "WARNING")

    def visual_command(self, parts):
        kind = " ".join(parts[1:]).lower() if len(parts) > 1 else "log"

        if "button" in kind:
            self.game.add_visual_script_template("Button Click")
        elif "damage" in kind:
            self.game.add_visual_script_template("Damage Self")
        else:
            self.game.add_visual_script_template("Log And Move")

    def plugin_command(self, parts):
        action = parts[1].lower() if len(parts) > 1 else "scan"

        if action == "scan":
            self.game.plugin_manager.scan()
            self.log(", ".join(self.game.plugin_manager.summary()) or "No plugins", "ENGINE")
        elif action == "hook" and len(parts) > 2:
            self.game.plugin_hook(parts[2])
        else:
            self.log("Uso: plugin scan | plugin hook on_editor_start", "WARNING")

    def example_command(self, parts):
        kind = parts[1].lower() if len(parts) > 1 else "ui"

        if kind in ("rpg", "actionrpg", "action_rpg"):
            self.game.create_example_action_rpg()
        elif kind == "ui":
            self.game.create_example_ui_scene()
        elif kind == "survival":
            self.game.create_template_survival()
        else:
            self.log("Uso: example ui/actionrpg/survival", "WARNING")

    def prefab_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: prefab save/apply/revert", "WARNING")
            return

        action = parts[1].lower()

        if action == "save":
            self.game.save_selected_prefab()
        elif action == "apply":
            self.game.prefab_workflow.apply_selected_to_prefab()
        elif action == "revert":
            self.game.prefab_workflow.revert_selected_prefab()
        else:
            self.log("Comando prefab inválido", "WARNING")

    def layout_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: layout save/reset/show", "WARNING")
            return

        action = parts[1].lower()

        if action == "save":
            self.game.save_editor_layout()
        elif action == "reset":
            self.game.reset_editor_layout()
        elif action == "show":
            self.game.show_all_panels()
        else:
            self.log("Comando layout inválido", "WARNING")

    def tag_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: tag add/remove/list nombre", "WARNING")
            return

        action = parts[1].lower()

        if action == "list":
            self.log(", ".join(self.game.tags), "ENGINE")
        elif action == "add" and len(parts) >= 3:
            self.game.tags_layers_manager.add_tag(parts[2])
            self.game.tags = self.game.tags_layers_manager.tags
        elif action == "remove" and len(parts) >= 3:
            self.game.tags_layers_manager.remove_tag(parts[2])
            self.game.tags = self.game.tags_layers_manager.tags
        else:
            self.log("Comando tag inválido", "WARNING")

    def layer_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: layer add/remove/list/visible/lock nombre", "WARNING")
            return

        action = parts[1].lower()

        if action == "list":
            self.log(", ".join(self.game.layers), "ENGINE")
        elif action == "add" and len(parts) >= 3:
            self.game.tags_layers_manager.add_layer(parts[2])
            self.game.layers = self.game.tags_layers_manager.layers
            self.game.layer_visibility.sync_layers()
        elif action == "remove" and len(parts) >= 3:
            self.game.tags_layers_manager.remove_layer(parts[2])
            self.game.layers = self.game.tags_layers_manager.layers
            self.game.layer_visibility.sync_layers()
        elif action == "visible" and len(parts) >= 3:
            self.game.layer_visibility.toggle_layer_visibility(parts[2])
        elif action == "lock" and len(parts) >= 3:
            self.game.layer_visibility.toggle_layer_lock(parts[2])
        else:
            self.log("Comando layer inválido", "WARNING")

    def show_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: show grid/gizmos/paths/names/colliders/chunks/coords/brush", "WARNING")
            return

        key_map = {
            "grid": "show_grid",
            "gizmos": "show_gizmos",
            "paths": "show_paths",
            "names": "show_names",
            "colliders": "show_colliders",
            "chunks": "show_chunks",
            "coords": "show_tile_coordinates",
            "brush": "show_brush_preview",
        }

        key = key_map.get(parts[1].lower())

        if not key:
            self.log("Opción show inválida", "WARNING")
            return

        self.game.editor_view_settings.toggle(key)

    def build_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: build get/set key value", "WARNING")
            return

        action = parts[1].lower()

        if action == "get":
            self.log(str(self.game.build_settings.data), "ENGINE")
        elif action == "set" and len(parts) >= 4:
            key = parts[2]
            value = " ".join(parts[3:])

            if value.lower() in ["true", "false"]:
                value = value.lower() == "true"
            elif value.isdigit():
                value = int(value)

            self.game.build_settings.set(key, value)
            self.log(f"Build setting {key} = {value}", "ENGINE")
        else:
            self.log("Comando build inválido", "WARNING")

    def rts_command(self, parts):
        if len(parts) < 2:
            self.log("Uso: rts attack/follow/guard/gather/cancel/formation", "WARNING")
            return

        action = parts[1].lower()

        if action == "cancel":
            self.game.command_system.cancel_units()
        elif action == "formation" and len(parts) >= 3:
            self.game.command_system.default_formation = parts[2].lower()
            self.log(f"Formation: {self.game.command_system.default_formation}", "RTS")
        elif action == "attack":
            import pygame
            mouse_grid = self.game.screen_to_grid(pygame.mouse.get_pos())
            self.game.command_system.attack_move_units(mouse_grid)
        else:
            self.log("Comando RTS inválido", "WARNING")

    def component_command(self, parts):
        if len(parts) < 3:
            self.log("Uso: component add Health/Team/Worker/ResourceNode/Collider2D", "WARNING")
            return

        if parts[1].lower() != "add":
            self.log("Solo se soporta: component add Nombre", "WARNING")
            return

        name = parts[2]

        if not hasattr(self.game, "component_registry"):
            self.log("ComponentRegistry no activo", "WARNING")
            return

        if not self.game.selected_units:
            self.log("No hay entidades seleccionadas", "WARNING")
            return

        added = 0

        for unit in self.game.selected_units:
            component = self.game.component_registry.create(name)

            if component:
                unit.add_component(component)
                added += 1

        self.game.history.take_snapshot(f"Add Component {name}")
        self.log(f"Componente {name} agregado a {added} entidad(es)", "ENGINE")

    def groups_command(self):
        for number, ids in self.game.control_groups.items():
            self.log(f"Grupo {number}: {len(ids)} entidad(es)", "RTS")

    def print_help(self):
        commands = [
            "help",
            "clear",
            "create script Nombre",
            "create component Nombre",
            "create system Nombre",
            "create scene Nombre",
            "create prefab Nombre",
            "create json Nombre",
            "create txt Nombre",
            "create folder Nombre",
            "browser open",
            "browser duplicate",
            "browser delete",
            "browser rename NuevoNombre",
            "browser rename_folder NuevoNombre",
            "browser tree",
            "browser refresh",
            "save",
            "load",
            "new_scene",
            "reload",
            "play",
            "editor",
            "version",
            "validate",
            "manifest",
            "tool select/move/entity/tile/obstacle/erase",
            "spawn player",
            "spawn 10 10",
            "delete selected",
            "duplicate",
            "rename selected NuevoNombre",
            "set selected speed 5",
            "brush size 1-5",
            "asset refresh/delete/rename/folder/open/tree/duplicate",
            "asset deps/import/graph",
            "ui label/button/progress/example",
            "visual log/button/damage",
            "plugin scan",
            "plugin hook on_editor_start",
            "example ui/actionrpg/survival",
            "prefab save/apply/revert",
            "tag list/add/remove",
            "layer list/add/remove/visible/lock",
            "show grid/gizmos/paths/names/colliders/chunks/coords/brush",
            "build get",
            "build set game_name MyGame",
            "rts cancel",
            "rts formation square/line/column/circle",
            "component add Health/Team/Worker/ResourceNode/Collider2D",
            "groups",
            "clear_logs",
        ]

        for command in commands:
            self.log(command, "INFO")

    def print_version(self):
        self.log(version_label(), "ENGINE")
        self.log(f"Engine version: {ENGINE_VERSION}", "ENGINE")
