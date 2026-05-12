import os
import shutil

from engine.asset_tools import AssetTools


class FileBrowser:
    """
    Content Browser / File Explorer Pro 0.6.0.

    Incluye:
    - project_path real
    - solo archivos del proyecto actual
    - carpetas separadas del engine
    - crear scripts/componentes/sistemas/json/txt/escenas/prefabs
    - crear carpetas normales y especiales
    - renombrar archivos con modal
    - renombrar carpetas con modal
    - duplicar assets
    - eliminar assets
    - abrir assets
    - abrir scripts/componentes/sistemas en ScriptEditor
    - abrir escenas
    - asignar sprites
    - usar prefabs seleccionados
    - drag & drop interno
    - click derecho con menú contextual
    - tree view de carpetas
    """

    def __init__(self, game):
        self.game = game

        self.assets = []
        self.folders = []

        self.selected_asset = None
        self.selected_folder = None

        self.scroll = 0
        self.folder_scroll = 0
        self.max_visible = 5

        self.filter_type = "All"
        self.tree_view = True

        self.dragging_asset = None
        self.drag_hover_folder = None

        self.last_created_path = None
        self.pending_delete_path = None
        self.pending_delete_kind = None

        # Context menu
        self.context_menu_open = False
        self.context_menu_pos = (0, 0)
        self.context_menu_target = None
        self.context_menu_items = []

        self.refresh()

    # =========================
    # PATHS
    # =========================

    def project_path(self):
        return getattr(
            self.game,
            "project_path",
            AssetTools.default_project_path()
        )

    def paths(self):
        return AssetTools.get_project_paths(self.project_path())

    def project_roots(self):
        paths = self.paths()

        return [
            paths["assets"],
            paths["sprites"],
            paths["audio"],
            paths["data"],
            paths["prefabs"],
            paths["scripts"],
            paths["components"],
            paths["systems"],
            paths["scenes"],
            paths["settings"],
            paths["plugins"],
        ]

    def allowed_folder_roots(self):
        paths = self.paths()

        return [
            paths["assets"],
            paths["scripts"],
            paths["components"],
            paths["systems"],
            paths["scenes"],
            paths["settings"],
            paths["plugins"],
        ]

    def ensure_valid_selected_folder(self):
        paths = self.paths()

        if not self.selected_folder:
            self.selected_folder = paths["assets"]

        if not os.path.exists(self.selected_folder):
            self.selected_folder = paths["assets"]

    def relative(self, path):
        try:
            return os.path.relpath(path, self.project_path())
        except Exception:
            return str(path)

    def is_inside_project(self, path):
        try:
            project_root = os.path.abspath(self.project_path())
            check_path = os.path.abspath(path)

            return os.path.commonpath([project_root, check_path]) == project_root

        except Exception:
            return False

    def can_modify_folder(self, folder):
        if not folder:
            return False

        if not self.is_inside_project(folder):
            return False

        project_root = os.path.abspath(self.project_path())
        folder_abs = os.path.abspath(folder)

        if folder_abs == project_root:
            return False

        return True

    # =========================
    # REFRESH / SCAN
    # =========================

    def refresh(self):
        AssetTools.ensure_project_folders(self.project_path())

        self.assets.clear()
        self.folders.clear()

        self.ensure_valid_selected_folder()
        self.scan_folders()

        paths = self.paths()

        self.scan_folder(
            paths["sprites"],
            "Sprite",
            [".png", ".jpg", ".jpeg", ".bmp", ".gif", ".webp"]
        )

        self.scan_folder(
            paths["audio"],
            "Audio",
            [".wav", ".mp3", ".ogg"]
        )

        self.scan_folder(
            paths["data"],
            "Data",
            [".json", ".txt", ".csv"]
        )

        self.scan_folder(
            paths["prefabs"],
            "Prefab",
            [".prefab"]
        )

        self.scan_folder(
            paths["scripts"],
            "Script",
            [".py"]
        )

        self.scan_folder(
            paths["components"],
            "Component",
            [".py"]
        )

        self.scan_folder(
            paths["systems"],
            "System",
            [".py"]
        )

        self.scan_folder(
            paths["scenes"],
            "Scene",
            [".scene"]
        )

        self.scan_folder(
            paths["settings"],
            "Settings",
            [".json"]
        )

        self.scan_folder(
            paths["plugins"],
            "Plugin",
            [".json", ".py"]
        )

        self.assets.sort(
            key=lambda asset: (
                asset["type"],
                asset["folder"].lower(),
                asset["filename"].lower()
            )
        )

        self.folders = sorted(list(dict.fromkeys(self.folders)))

        if self.selected_asset:
            selected_path = self.selected_asset.get("path", "")

            if not os.path.exists(selected_path):
                self.selected_asset = None

        if self.selected_folder and not os.path.exists(self.selected_folder):
            self.selected_folder = self.paths()["assets"]

        self.clamp_scroll()
        self.clamp_folder_scroll()

    def scan_folders(self):
        for root in self.project_roots():
            os.makedirs(root, exist_ok=True)

        for root in self.allowed_folder_roots():
            if not os.path.exists(root):
                os.makedirs(root, exist_ok=True)

            for folder, dirs, _ in os.walk(root):
                dirs[:] = [
                    directory for directory in dirs
                    if directory not in ["__pycache__", ".git"]
                    and not directory.startswith(".")
                ]

                if "__pycache__" in folder:
                    continue

                if ".git" in folder:
                    continue

                if folder not in self.folders:
                    self.folders.append(folder)

    def scan_folder(self, folder, asset_type, extensions):
        if not os.path.exists(folder):
            os.makedirs(folder, exist_ok=True)

        for root, dirs, files in os.walk(folder):
            dirs[:] = [
                directory for directory in dirs
                if directory not in ["__pycache__", ".git"]
                and not directory.startswith(".")
            ]

            for filename in files:
                if filename.startswith("."):
                    continue

                ext = os.path.splitext(filename)[1].lower()

                if ext not in extensions:
                    continue

                path = os.path.join(root, filename)

                if not self.is_inside_project(path):
                    continue

                name = os.path.splitext(filename)[0]

                self.assets.append(
                    {
                        "name": name,
                        "filename": filename,
                        "path": path,
                        "type": asset_type,
                        "extension": ext,
                        "folder": root,
                        "relative_path": os.path.relpath(
                            path,
                            self.project_path()
                        ),
                    }
                )

    # =========================
    # FILTER / SCROLL
    # =========================

    def get_visible_assets(self):
        self.ensure_valid_selected_folder()

        assets = self.assets

        if self.selected_folder:
            selected_folder = os.path.abspath(self.selected_folder)
            assets = [
                asset for asset in assets
                if self.path_is_inside(os.path.abspath(asset.get("folder", "")), selected_folder)
            ]

        if self.filter_type != "All":
            assets = [
                asset for asset in assets
                if asset["type"] == self.filter_type
            ]

        return assets

    def path_is_inside(self, path, folder):
        try:
            return os.path.commonpath([folder, path]) == folder
        except Exception:
            return False

    def cycle_filter(self):
        filters = [
            "All",
            "Sprite",
            "Audio",
            "Data",
            "Prefab",
            "Script",
            "Component",
            "System",
            "Scene",
            "Settings",
            "Plugin",
        ]

        index = filters.index(self.filter_type) if self.filter_type in filters else 0

        self.filter_type = filters[(index + 1) % len(filters)]
        self.scroll = 0

        self.safe_log(f"Content filter: {self.filter_type}", "ASSET")

    def set_filter(self, filter_type):
        valid_filters = [
            "All",
            "Sprite",
            "Audio",
            "Data",
            "Prefab",
            "Script",
            "Component",
            "System",
            "Scene",
            "Settings",
            "Plugin",
        ]

        if filter_type not in valid_filters:
            filter_type = "All"

        self.filter_type = filter_type
        self.scroll = 0

    def toggle_tree_view(self):
        self.tree_view = not self.tree_view
        self.safe_log(f"Tree View: {self.tree_view}", "ASSET")

    def scroll_up(self):
        self.scroll = max(0, self.scroll - 1)

    def scroll_down(self):
        assets = self.get_visible_assets()
        max_scroll = max(0, len(assets) - self.max_visible)
        self.scroll = min(max_scroll, self.scroll + 1)

    def folder_scroll_up(self):
        self.folder_scroll = max(0, self.folder_scroll - 1)

    def folder_scroll_down(self):
        max_scroll = max(0, len(self.folders) - 8)
        self.folder_scroll = min(max_scroll, self.folder_scroll + 1)

    def clamp_scroll(self):
        assets = self.get_visible_assets()
        max_scroll = max(0, len(assets) - self.max_visible)
        self.scroll = max(0, min(self.scroll, max_scroll))

    def clamp_folder_scroll(self):
        max_scroll = max(0, len(self.folders) - 8)
        self.folder_scroll = max(0, min(self.folder_scroll, max_scroll))

    # =========================
    # SELECT
    # =========================

    def select_asset_by_index(self, visible_index):
        assets = self.get_visible_assets()
        index = self.scroll + visible_index

        if index < 0 or index >= len(assets):
            return False

        self.selected_asset = assets[index]

        self.safe_log(
            f"Asset seleccionado: {self.selected_asset['filename']}",
            "ASSET"
        )

        return True

    def select_folder_by_index(self, visible_index):
        index = self.folder_scroll + visible_index

        if index < 0 or index >= len(self.folders):
            return False

        self.selected_folder = self.folders[index]
        self.scroll = 0

        self.safe_log(
            f"Folder seleccionado: {self.relative(self.selected_folder)}",
            "ASSET"
        )

        return True

    def selected_path(self):
        if not self.selected_asset:
            return None

        return self.selected_asset.get("path")

    def selected_folder_path(self):
        return self.selected_folder

    # =========================
    # OPEN
    # =========================

    def open_selected(self):
        asset = self.selected_asset

        if not asset:
            self.safe_log("No hay asset seleccionado", "WARNING")
            return False

        asset_type = asset.get("type")

        if asset_type in ["Script", "Component", "System"]:
            self.open_script_like_file(asset)
            return True

        if asset_type == "Scene":
            self.open_scene(asset)
            return True

        if asset_type == "Prefab":
            self.game.active_tool = "Entity"
            self.safe_log(
                "Prefab listo. Usa herramienta Entity para colocarlo.",
                "ASSET"
            )
            return True

        if asset_type == "Sprite":
            if hasattr(self.game, "assign_selected_sprite"):
                self.game.assign_selected_sprite()
                return True

        if asset_type in ["Data", "Settings", "Plugin"]:
            self.open_text_file(asset)
            return True

        self.safe_log(f"Open no implementado para: {asset_type}", "WARNING")
        return False

    def open_script_like_file(self, asset):
        if not hasattr(self.game, "script_editor"):
            self.safe_log("ScriptEditor no disponible", "ERROR")
            return

        path = asset.get("path")

        if not path or not os.path.exists(path):
            self.safe_log("El archivo no existe", "ERROR")
            return

        if hasattr(self.game.script_editor, "open_file"):
            self.game.script_editor.open_file(path)
            self.game.script_editor.visible = True
            return

        try:
            with open(path, "r", encoding="utf-8") as file:
                content = file.read()

            self.game.script_editor.filename = asset["filename"]
            self.game.script_editor.lines = content.splitlines() or [""]
            self.game.script_editor.visible = True

        except Exception as error:
            self.safe_log(f"No se pudo abrir archivo: {error}", "ERROR")

    def open_text_file(self, asset):
        self.open_script_like_file(asset)

    def open_scene(self, asset):
        if not hasattr(self.game, "scene_manager"):
            self.safe_log("SceneManager no disponible", "ERROR")
            return

        scene_name = asset.get("filename")

        if not scene_name:
            self.safe_log("Scene inválida", "ERROR")
            return

        self.game.scene_manager.current_scene = scene_name

        if hasattr(self.game.scene_manager, "current_scene_path"):
            self.game.scene_manager.current_scene_path = asset.get("path")

        self.game.load_scene()

    # =========================
    # CREATE FILES
    # =========================

    def create_folder(self, name=None):
        self.ensure_valid_selected_folder()

        base = self.selected_folder or self.paths()["assets"]

        if not self.is_inside_project(base):
            base = self.paths()["assets"]

        name = AssetTools.safe_name(name, "NewFolder")
        path = os.path.join(base, name)

        if os.path.exists(path):
            index = 1

            while os.path.exists(os.path.join(base, f"{name}_{index}")):
                index += 1

            path = os.path.join(base, f"{name}_{index}")

        try:
            os.makedirs(path, exist_ok=True)

            self.last_created_path = path
            self.selected_folder = path

            self.refresh()

            self.safe_log(
                f"Carpeta creada: {self.relative(path)}",
                "ASSET"
            )

            return path

        except Exception as error:
            self.safe_log(f"No se pudo crear carpeta: {error}", "ERROR")
            return None

    def create_special_folder(self, folder_type):
        try:
            path = AssetTools.create_special_folder(
                self.project_path(),
                folder_type
            )

            self.last_created_path = path
            self.selected_folder = path

            self.refresh()

            self.safe_log(
                f"Carpeta especial lista: {self.relative(path)}",
                "ASSET"
            )

            return path

        except Exception as error:
            self.safe_log(
                f"No se pudo crear carpeta especial: {error}",
                "ERROR"
            )
            return None

    def create_script(self, name="NewScript"):
        try:
            path = AssetTools.create_script(self.project_path(), name)

            self.last_created_path = path
            self.selected_folder = os.path.dirname(path)

            self.refresh()
            self.select_asset_by_path(path)

            self.safe_log(
                f"Script creado: {self.relative(path)}",
                "SCRIPT"
            )

            return path

        except Exception as error:
            self.safe_log(f"No se pudo crear script: {error}", "ERROR")
            return None

    def create_component(self, name="NewComponent"):
        try:
            path = AssetTools.create_component(self.project_path(), name)

            self.last_created_path = path
            self.selected_folder = os.path.dirname(path)

            self.refresh()
            self.select_asset_by_path(path)

            self.safe_log(
                f"Componente creado: {self.relative(path)}",
                "ENGINE"
            )

            return path

        except Exception as error:
            self.safe_log(f"No se pudo crear componente: {error}", "ERROR")
            return None

    def create_system(self, name="NewSystem"):
        try:
            path = AssetTools.create_system(self.project_path(), name)

            self.last_created_path = path
            self.selected_folder = os.path.dirname(path)

            self.refresh()
            self.select_asset_by_path(path)

            self.safe_log(
                f"Sistema creado: {self.relative(path)}",
                "ENGINE"
            )

            return path

        except Exception as error:
            self.safe_log(f"No se pudo crear sistema: {error}", "ERROR")
            return None

    def create_json(self, name="NewData"):
        try:
            target = self.selected_folder or self.paths()["data"]

            if not self.is_inside_project(target):
                target = self.paths()["data"]

            path = AssetTools.create_json(self.project_path(), target, name)

            self.last_created_path = path
            self.selected_folder = os.path.dirname(path)

            self.refresh()
            self.select_asset_by_path(path)

            self.safe_log(
                f"JSON creado: {self.relative(path)}",
                "ASSET"
            )

            return path

        except Exception as error:
            self.safe_log(f"No se pudo crear JSON: {error}", "ERROR")
            return None

    def create_txt(self, name="NewText"):
        try:
            target = self.selected_folder or self.paths()["data"]

            if not self.is_inside_project(target):
                target = self.paths()["data"]

            path = AssetTools.create_txt(self.project_path(), target, name)

            self.last_created_path = path
            self.selected_folder = os.path.dirname(path)

            self.refresh()
            self.select_asset_by_path(path)

            self.safe_log(
                f"TXT creado: {self.relative(path)}",
                "ASSET"
            )

            return path

        except Exception as error:
            self.safe_log(f"No se pudo crear TXT: {error}", "ERROR")
            return None

    def create_scene(self, name="NewScene"):
        try:
            path = AssetTools.create_scene(self.project_path(), name)

            self.last_created_path = path
            self.selected_folder = os.path.dirname(path)

            self.refresh()
            self.select_asset_by_path(path)

            self.safe_log(
                f"Escena creada: {self.relative(path)}",
                "SCENE"
            )

            return path

        except Exception as error:
            self.safe_log(f"No se pudo crear escena: {error}", "ERROR")
            return None

    def create_prefab(self, name="NewPrefab"):
        try:
            path = AssetTools.create_prefab(self.project_path(), name)

            self.last_created_path = path
            self.selected_folder = os.path.dirname(path)

            self.refresh()
            self.select_asset_by_path(path)

            self.safe_log(
                f"Prefab creado: {self.relative(path)}",
                "ASSET"
            )

            return path

        except Exception as error:
            self.safe_log(f"No se pudo crear prefab: {error}", "ERROR")
            return None

    # =========================
    # FIND / SELECT BY PATH
    # =========================

    def select_asset_by_path(self, path):
        normalized = os.path.normpath(path)

        for asset in self.assets:
            asset_path = os.path.normpath(asset.get("path", ""))

            if asset_path == normalized:
                self.selected_asset = asset

                visible_assets = self.get_visible_assets()

                if asset in visible_assets:
                    index = visible_assets.index(asset)
                    self.scroll = max(0, min(index, max(0, len(visible_assets) - self.max_visible)))

                return True

        return False

    def find_asset_by_name(self, name):
        query = str(name).lower()

        for asset in self.assets:
            if asset["name"].lower() == query:
                return asset

            if asset["filename"].lower() == query:
                return asset

        return None

    # =========================
    # DUPLICATE / DELETE / RENAME
    # =========================

    def duplicate_selected_asset(self):
        asset = self.selected_asset

        if not asset:
            self.safe_log("No hay asset seleccionado", "WARNING")
            return None

        path = asset.get("path")

        if not path or not os.path.exists(path):
            self.safe_log("El asset no existe", "WARNING")
            return None

        if not self.is_inside_project(path):
            self.safe_log("No se puede duplicar fuera del proyecto", "ERROR")
            return None

        folder = os.path.dirname(path)
        filename = os.path.basename(path)

        name, ext = os.path.splitext(filename)
        target = AssetTools.unique_path(folder, f"{name}_copy{ext}")

        try:
            shutil.copy2(path, target)

            self.last_created_path = target
            self.selected_folder = folder

            self.refresh()
            self.select_asset_by_path(target)

            self.safe_log(
                f"Asset duplicado: {self.relative(target)}",
                "ASSET"
            )

            return target

        except Exception as error:
            self.safe_log(f"No se pudo duplicar asset: {error}", "ERROR")
            return None

    def delete_selected_asset(self, confirm=False):
        asset = self.selected_asset

        if not asset:
            self.safe_log("No hay asset seleccionado", "WARNING")
            return False

        path = asset.get("path")

        if not path or not os.path.exists(path):
            self.safe_log("El asset no existe", "WARNING")
            return False

        if not self.is_inside_project(path):
            self.safe_log("No puedes eliminar archivos fuera del proyecto", "ERROR")
            return False

        if not confirm and not self.delete_confirmed(path, "asset"):
            self.safe_log(
                f"Confirmar eliminación de asset: {self.relative(path)}. Ejecuta Delete otra vez.",
                "WARNING"
            )
            return False

        try:
            os.remove(path)

            self.selected_asset = None
            self.clear_pending_delete()
            self.refresh()

            self.safe_log(
                f"Asset eliminado: {self.relative(path)}",
                "ASSET"
            )

            return True

        except Exception as error:
            self.safe_log(f"No se pudo eliminar asset: {error}", "ERROR")
            return False

    def rename_selected_asset(self, new_name=None):
        asset = self.selected_asset

        if not asset:
            self.safe_log("No hay asset seleccionado", "WARNING")
            return None

        if not new_name:
            self.safe_log("Falta nuevo nombre para renombrar", "WARNING")
            return None

        old_path = asset.get("path")

        if not old_path or not os.path.exists(old_path):
            self.safe_log("El asset no existe", "WARNING")
            return None

        if not self.is_inside_project(old_path):
            self.safe_log("No puedes renombrar archivos fuera del proyecto", "ERROR")
            return None

        folder = os.path.dirname(old_path)
        ext = os.path.splitext(old_path)[1]

        new_name = AssetTools.safe_name(new_name, "RenamedAsset")

        if not new_name.endswith(ext):
            new_name += ext

        new_path = os.path.join(folder, new_name)

        if os.path.exists(new_path):
            self.safe_log("Ya existe un asset con ese nombre", "WARNING")
            return None

        try:
            os.rename(old_path, new_path)

            self.refresh()
            self.select_asset_by_path(new_path)

            self.safe_log(
                f"Asset renombrado: {self.relative(new_path)}",
                "ASSET"
            )

            return new_path

        except Exception as error:
            self.safe_log(f"No se pudo renombrar asset: {error}", "ERROR")
            return None

    def rename_selected_folder(self, new_name):
        if not self.selected_folder:
            self.safe_log("No hay carpeta seleccionada", "WARNING")
            return None

        old_path = self.selected_folder

        if not os.path.exists(old_path):
            self.safe_log("La carpeta no existe", "WARNING")
            return None

        if not self.can_modify_folder(old_path):
            self.safe_log("No puedes renombrar esa carpeta", "WARNING")
            return None

        new_name = AssetTools.safe_name(new_name, "RenamedFolder")

        parent = os.path.dirname(old_path)
        new_path = os.path.join(parent, new_name)

        if os.path.exists(new_path):
            index = 1

            while os.path.exists(os.path.join(parent, f"{new_name}_{index}")):
                index += 1

            new_path = os.path.join(parent, f"{new_name}_{index}")

        try:
            os.rename(old_path, new_path)

            self.selected_folder = new_path

            self.refresh()

            self.safe_log(
                f"Carpeta renombrada: {self.relative(new_path)}",
                "ASSET"
            )

            return new_path

        except Exception as error:
            self.safe_log(f"No se pudo renombrar carpeta: {error}", "ERROR")
            return None

    def delete_selected_folder(self, confirm=False):
        if not self.selected_folder:
            self.safe_log("No hay carpeta seleccionada", "WARNING")
            return False

        folder = self.selected_folder

        if not self.can_modify_folder(folder):
            self.safe_log("No puedes eliminar esa carpeta", "WARNING")
            return False

        if self.is_critical_folder(folder) and not confirm and not self.delete_confirmed(folder, "folder"):
            self.safe_log(
                f"Carpeta crítica: {self.relative(folder)}. Ejecuta Delete otra vez para confirmar.",
                "WARNING"
            )
            return False

        if not confirm and not self.delete_confirmed(folder, "folder"):
            self.safe_log(
                f"Confirmar eliminación de carpeta: {self.relative(folder)}. Ejecuta Delete otra vez.",
                "WARNING"
            )
            return False

        try:
            shutil.rmtree(folder)

            self.selected_folder = self.paths()["assets"]
            self.selected_asset = None
            self.clear_pending_delete()

            self.refresh()

            self.safe_log(
                f"Carpeta eliminada: {self.relative(folder)}",
                "ASSET"
            )

            return True

        except Exception as error:
            self.safe_log(f"No se pudo eliminar carpeta: {error}", "ERROR")
            return False

    def is_critical_folder(self, folder):
        folder_abs = os.path.abspath(folder)
        critical = {
            os.path.abspath(path)
            for key, path in self.paths().items()
            if key in ["assets", "scripts", "scenes", "root_scenes", "logs", "settings"]
        }
        return folder_abs in critical

    def delete_confirmed(self, path, kind):
        path = os.path.abspath(path)

        if self.pending_delete_path == path and self.pending_delete_kind == kind:
            return True

        self.pending_delete_path = path
        self.pending_delete_kind = kind
        return False

    def clear_pending_delete(self):
        self.pending_delete_path = None
        self.pending_delete_kind = None

    # =========================
    # DRAG & DROP INTERNAL
    # =========================

    def start_drag_selected(self):
        if not self.selected_asset:
            return False

        self.dragging_asset = self.selected_asset
        return True

    def cancel_drag(self):
        self.dragging_asset = None
        self.drag_hover_folder = None

    def set_drag_hover_folder_by_index(self, visible_index):
        index = self.folder_scroll + visible_index

        if index < 0 or index >= len(self.folders):
            self.drag_hover_folder = None
            return False

        self.drag_hover_folder = self.folders[index]
        return True

    def drop_dragged_asset(self):
        if not self.dragging_asset or not self.drag_hover_folder:
            self.cancel_drag()
            return False

        old_path = self.dragging_asset.get("path")

        if not old_path or not os.path.exists(old_path):
            self.cancel_drag()
            return False

        if not self.is_inside_project(old_path):
            self.safe_log("No puedes mover assets fuera del proyecto", "ERROR")
            self.cancel_drag()
            return False

        if not self.is_inside_project(self.drag_hover_folder):
            self.safe_log("Destino inválido fuera del proyecto", "ERROR")
            self.cancel_drag()
            return False

        new_path = os.path.join(
            self.drag_hover_folder,
            os.path.basename(old_path)
        )

        if old_path == new_path:
            self.cancel_drag()
            return False

        if os.path.exists(new_path):
            self.safe_log(
                "Ya existe un archivo con ese nombre en esa carpeta",
                "WARNING"
            )
            self.cancel_drag()
            return False

        try:
            shutil.move(old_path, new_path)

            self.safe_log(
                f"Asset movido a: {self.relative(self.drag_hover_folder)}",
                "ASSET"
            )

            self.cancel_drag()

            self.selected_folder = os.path.dirname(new_path)

            self.refresh()
            self.select_asset_by_path(new_path)

            return True

        except Exception as error:
            self.safe_log(f"No se pudo mover asset: {error}", "ERROR")
            self.cancel_drag()
            return False

    # =========================
    # CONTEXT MENU / RIGHT CLICK
    # =========================

    def open_context_menu(self, pos, target="empty"):
        self.context_menu_open = True
        self.context_menu_pos = pos
        self.context_menu_target = target
        self.context_menu_items = self.get_context_menu_items(target)

    def close_context_menu(self):
        self.context_menu_open = False
        self.context_menu_target = None
        self.context_menu_items = []

    def get_context_menu_items(self, target):
        common_create = [
            ("New Script", "create_script"),
            ("New Component", "create_component"),
            ("New System", "create_system"),
            ("New Scene", "create_scene"),
            ("New Prefab", "create_prefab"),
            ("New JSON", "create_json"),
            ("New TXT", "create_txt"),
            ("New Folder", "create_folder"),
        ]

        if target == "asset":
            return [
                ("Open", "open"),
                ("Duplicate", "duplicate_asset"),
                ("Rename Asset", "rename_asset"),
                ("Delete Asset", "delete_asset"),
                ("Refresh", "refresh"),
                ("---", None),
            ] + common_create

        if target == "folder":
            return [
                ("Rename Folder", "rename_folder"),
                ("Delete Folder", "delete_folder"),
                ("New Folder", "create_folder"),
                ("Refresh", "refresh"),
                ("---", None),
            ] + common_create

        return common_create + [
            ("Refresh", "refresh"),
            ("Tree View", "tree"),
        ]

    def execute_context_action(self, action):
        if not action:
            return

        if action == "open":
            self.open_selected()

        elif action == "duplicate_asset":
            self.duplicate_selected_asset()

        elif action == "rename_asset":
            if hasattr(self.game, "open_create_modal"):
                self.game.open_create_modal("rename_asset")
            else:
                self.rename_selected_asset("RenamedAsset")

        elif action == "delete_asset":
            self.delete_selected_asset()

        elif action == "rename_folder":
            if hasattr(self.game, "open_create_modal"):
                self.game.open_create_modal("rename_folder")
            else:
                self.rename_selected_folder("RenamedFolder")

        elif action == "delete_folder":
            self.delete_selected_folder()

        elif action == "create_script":
            if hasattr(self.game, "open_create_modal"):
                self.game.open_create_modal("create_script")
            else:
                self.create_script("NewScript")

        elif action == "create_component":
            if hasattr(self.game, "open_create_modal"):
                self.game.open_create_modal("create_component")
            else:
                self.create_component("NewComponent")

        elif action == "create_system":
            if hasattr(self.game, "open_create_modal"):
                self.game.open_create_modal("create_system")
            else:
                self.create_system("NewSystem")

        elif action == "create_scene":
            if hasattr(self.game, "open_create_modal"):
                self.game.open_create_modal("create_scene")
            else:
                self.create_scene("NewScene")

        elif action == "create_prefab":
            if hasattr(self.game, "open_create_modal"):
                self.game.open_create_modal("create_prefab")
            else:
                self.create_prefab("NewPrefab")

        elif action == "create_json":
            if hasattr(self.game, "open_create_modal"):
                self.game.open_create_modal("create_json")
            else:
                self.create_json("NewData")

        elif action == "create_txt":
            if hasattr(self.game, "open_create_modal"):
                self.game.open_create_modal("create_txt")
            else:
                self.create_txt("NewText")

        elif action == "create_folder":
            if hasattr(self.game, "open_create_modal"):
                self.game.open_create_modal("create_folder")
            else:
                self.create_folder("NewFolder")

        elif action == "refresh":
            if hasattr(self.game, "refresh_project"):
                self.game.refresh_project()
            else:
                self.refresh()

        elif action == "tree":
            self.toggle_tree_view()

        self.close_context_menu()

    # =========================
    # HELPERS
    # =========================

    def get_selected_type(self):
        if not self.selected_asset:
            return None

        return self.selected_asset.get("type")

    def get_selected_name(self):
        if not self.selected_asset:
            return None

        return self.selected_asset.get("name")

    def get_selected_filename(self):
        if not self.selected_asset:
            return None

        return self.selected_asset.get("filename")

    def has_selected_asset(self):
        return self.selected_asset is not None

    def has_selected_folder(self):
        return self.selected_folder is not None

    def safe_log(self, message, level="INFO"):
        if hasattr(self.game, "console"):
            self.game.console.log(message, level)

    def debug_print_assets(self):
        for asset in self.assets:
            print(asset)

    def debug_print_folders(self):
        for folder in self.folders:
            print(folder)
