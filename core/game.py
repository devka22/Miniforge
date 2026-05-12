import os
import subprocess
import sys
import pygame

from map.grid import Grid
from entities.unit import Unit
from entities.game_object import GameObject
from input.input_handler import InputHandler
from render.renderer import Renderer

from engine.world import World
from engine.event_bus import EventBus
from engine.camera import Camera
from engine.script_manager import ScriptManager
from engine.resource_manager import ResourceManager
from engine.project_settings import ProjectSettings
from engine.developer_console import DeveloperConsole
from engine.editor_history import EditorHistory
from engine.script_editor import ScriptEditor
from engine.project_browser import ProjectBrowser
from engine.asset_tools import AssetTools
from engine.asset_database import AssetDatabase
from engine.file_browser import FileBrowser
from engine.prefab_manager import PrefabManager
from engine.scene_manager import SceneManager
from engine.inspector_editor import InspectorEditor
from engine.autosave_manager import AutosaveManager
from engine.project_manager import ProjectManager
from engine.scene_hierarchy import SceneHierarchy
from engine.error_handler import ErrorHandler
from engine.game_api import GameAPI

from engine.scene_tools import SceneTools
from engine.asset_operations import AssetOperations
from engine.prefab_workflow import PrefabWorkflow

from engine.layout_manager import LayoutManager
from engine.play_mode_manager import PlayModeManager
from engine.build_settings import BuildSettings
from engine.scene_validator import SceneValidator
from engine.manifest_builder import ManifestBuilder
from engine.runtime_exporter import RuntimeExporter
from engine.view_mode import ViewMode

from engine.runtime_config import RuntimeConfig
from engine.tags_layers_manager import TagsLayersManager
from engine.layer_visibility import LayerVisibility
from engine.editor_view_settings import EditorViewSettings
from engine.selection_manager import SelectionManager
from engine.rts_command_queue import RTSCommandQueue
from engine.component_registry import ComponentRegistry
from engine.editor_tools import EditorTools

from engine.create_asset_modal import CreateAssetModal
from engine.project_validator import ProjectValidator
from engine.project_templates import ProjectTemplates
from engine.input_map import InputMap
from engine.build_profiles import BuildProfiles
from engine.plugin_manager import PluginManager
from engine.asset_reference_resolver import AssetReferenceResolver
from engine.hierarchy_manager import HierarchyManager
from engine.prefab_overrides import PrefabOverrides
from engine.editor_tabs import EditorTabs
from engine.command_palette import CommandPalette
from engine.diagnostics import Diagnostics
from engine.theme import Theme
from engine.system_scheduler import SystemScheduler
from engine.visual_input_editor import VisualInputEditor
from engine.scene_view_tools import SceneViewTools
from engine.tilemap_layers import TilemapLayers
from engine.audio_mixer import AudioMixer
from engine.component_tools import ComponentTools
from engine.build_report import BuildReport
from engine.animation_graph import AnimationGraphLibrary
from engine.ui_canvas import UICanvas
from engine.visual_scripting import VisualScriptRuntime
from engine.advanced_prefabs import AdvancedPrefabSystem
from engine.profiler import Profiler
from engine.upgrade_manifest import EngineUpgradeManifest
from engine.version import ENGINE_VERSION, version_label
from core.engine_config import EngineConfig

from systems.input_system import InputSystem
from systems.movement_system import MovementSystem
from systems.render_system import RenderSystem
from systems.command_system import CommandSystem
from systems.camera_system import CameraSystem
from systems.editor_system import EditorSystem
from systems.physics_system import PhysicsSystem
from systems.audio_system import AudioSystem
from systems.animation_system import AnimationSystem
from systems.ui_system import UISystem
from systems.visual_script_system import VisualScriptSystem
from systems.gameplay_system import GameplaySystem

from engine.ui.menu_bar import MenuBar
from engine.ui.toolbar import Toolbar


class Game:
    """
    MiniForge 0.6.0 Alpha
    Developer Workflow & Modular Projects Build.

    Esta versión usa project_path real:

    projects/DefaultProject/
    ├── assets/
    ├── scripts/
    ├── components/
    ├── systems/
    ├── scenes/
    ├── settings/
    ├── logs/
    ├── plugins/
    └── builds/
    """

    def __init__(self, runtime_mode=False):
        pygame.init()

        self.runtime_mode = runtime_mode or os.environ.get("MINIFORGE_RUNTIME") == "1"

        # =========================
        # SETTINGS / PROJECT PATH
        # =========================

        self.settings = ProjectSettings()
        self.settings.load()

        default_project_path = AssetTools.default_project_path()
        current_directory = os.getcwd()

        if (
            self.settings.get("project_path") is None
            and (
                os.path.exists(os.path.join(current_directory, "project", "project.json"))
                or os.path.exists(os.path.join(current_directory, "saves", "scenes"))
            )
        ):
            default_project_path = current_directory

        self.project_path = self.settings.get(
            "project_path",
            default_project_path
        )

        self.project_path = os.path.normpath(self.project_path)

        AssetTools.ensure_project_folders(self.project_path)
        self.engine_config = EngineConfig(self.project_path)

        self.project_paths = AssetTools.get_project_paths(self.project_path)

        width = self.settings.get("window_width", 1100)
        height = self.settings.get("window_height", 740)

        self.screen = pygame.display.set_mode((width, height))

        pygame.display.set_caption(
            self.settings.get(
                "project_name",
                f"{version_label()} - Beta Editor"
            )
        )

        self.clock = pygame.time.Clock()
        self.running = True
        self.mode = "PLAY" if self.runtime_mode else self.settings.get("start_mode", "EDITOR")

        # =========================
        # EDITOR STATE
        # =========================

        self.active_tool = "Select"
        self.tile_brush = 0
        self.brush_size = 1

        self.left_panel_scroll = 0
        self.navigator_scroll = 0
        self.navigator_max_scroll = 900
        self.navigator_hover_item = None

        self.navigator_search_active = False
        self.navigator_search_text = ""

        self.navigator_sections_open = {
            "Scene": True,
            "Entity": True,
            "Assets": True,
            "Create": True,
            "Scripts": False,
            "Prefabs": False,
            "Map": False,
            "Components": False,
            "UI": False,
            "Visual": False,
            "RTS": False,
            "Runtime": False,
            "Plugins": False,
            "Layout": False,
            "Settings": False,
        }

        self.navigator_icons = {
            "Scene": "SCN",
            "Entity": "ENT",
            "Assets": "AST",
            "Create": "NEW",
            "Scripts": "PY",
            "Prefabs": "PFB",
            "Map": "MAP",
            "Components": "CMP",
            "UI": "UI",
            "Visual": "VS",
            "RTS": "RTS",
            "Runtime": "RUN",
            "Plugins": "PLG",
            "Layout": "LAY",
            "Settings": "CFG",
        }

        self.toolbar_compact = True
        self.menu_dropdown_open = None

        self.active_settings_panel = None
        self.settings_editing_key = None
        self.settings_edit_buffer = ""

        self.inspector_sections_open = {
            "Transform": True,
            "Movement": True,
            "Entity": True,
            "Render": True,
            "RTS": True,
            "Components": True,
            "Scripts": False,
            "Prefab": True,
            "Debug": False,
        }

        self.last_visual_error = ""
        self.last_visual_warning = ""
        self.last_script_reload_check = 0.0
        self.scene_dirty = False
        self.scene_dirty_reason = ""
        self.active_editor_tab = "Scene"
        self.editor_tabs = EditorTabs()
        self.theme = Theme()

        # =========================
        # CORE SERVICES
        # =========================

        self.console = DeveloperConsole()
        self.console.attach_game(self)

        self.console.log(f"Project path: {self.project_path}", "ENGINE")

        self.error_handler = ErrorHandler(self)
        self.api = GameAPI(self)
        self.asset_resolver = AssetReferenceResolver(self)
        self.command_palette = CommandPalette(self)
        self.diagnostics = Diagnostics(self)
        self.visual_input_editor = VisualInputEditor(self)
        self.scene_view_tools = SceneViewTools(self)
        self.audio_mixer = AudioMixer(self)
        self.animation_graphs = AnimationGraphLibrary()
        self.visual_script_runtime = VisualScriptRuntime(self)
        self.profiler = Profiler(self)
        self.upgrade_manifest = EngineUpgradeManifest()

        self.asset_database = AssetDatabase(
            root=self.project_asset_path(),
            project_root=self.project_path
        )
        self.input_map = InputMap(self.project_join("settings", "input_map.json"))
        self.plugin_manager = PluginManager(self.project_path)

        self.resources = ResourceManager(self.project_asset_path())
        self.safe_scan_resources()

        self.script_manager = ScriptManager()
        self.safe_scan_scripts()

        self.script_editor = ScriptEditor(self)
        self.project_browser = ProjectBrowser(self)
        self.file_browser = FileBrowser(self)

        self.create_asset_modal = CreateAssetModal(self)
        self.project_validator = ProjectValidator(self)

        # =========================
        # MAP / WORLD / CAMERA
        # =========================

        self.grid = Grid(
            width=self.settings.get("grid_width", 60),
            height=self.settings.get("grid_height", 40),
            tile_size=self.settings.get("tile_size", 32),
            chunk_size=8
        )
        self.tilemap_layers = TilemapLayers(self.grid.width, self.grid.height)
        self.ui_canvas = UICanvas(self)

        self.world = World()
        self.event_bus = EventBus()
        self.camera = Camera()

        map_width = self.grid.width * self.grid.tile_size
        map_height = self.grid.height * self.grid.tile_size

        self.camera.set_bounds(0, 0, map_width, map_height)
        self.camera.set_viewport(self.get_world_viewport_rect())

        # =========================
        # ENTITIES
        # =========================

        self.selected_units = []

        self.units = [
            Unit(2, 2, self),
            Unit(4, 4, self),
            Unit(6, 6, self),
        ]

        self.world.entities = self.units

        # =========================
        # INPUT / RENDER
        # =========================

        self.input_handler = InputHandler(self)
        self.renderer = Renderer(self)

        # =========================
        # SYSTEMS
        # =========================

        self.command_system = CommandSystem(self)

        self.systems = [
            InputSystem(self.input_handler),
            MovementSystem(self),
            AnimationSystem(self),
            VisualScriptSystem(self),
            GameplaySystem(self),
            PhysicsSystem(self),
            AudioSystem(self),
            UISystem(self),
            CameraSystem(self),
            EditorSystem(self),
        ]
        self.system_scheduler = SystemScheduler(self)

        for priority, system in enumerate(self.systems):
            self.system_scheduler.register(system, priority=priority * 10)

        self.render_system = RenderSystem(self.renderer)
        self.project_runtime_systems = []

        # =========================
        # EDITOR MANAGERS
        # =========================

        self.scene_manager = SceneManager(self)
        self.inspector_editor = InspectorEditor(self)
        self.autosave_manager = AutosaveManager(self, interval_seconds=60)
        self.project_manager = ProjectManager(self)
        self.scene_hierarchy = SceneHierarchy(self)

        self.scene_tools = SceneTools(self)
        self.asset_operations = AssetOperations(self)
        self.prefab_workflow = PrefabWorkflow(self)
        self.advanced_prefabs = AdvancedPrefabSystem(self)
        self.hierarchy_manager = HierarchyManager(self)
        self.prefab_overrides = PrefabOverrides(self)

        self.layout_manager = LayoutManager(self)
        self.play_mode_manager = PlayModeManager(self)
        self.build_settings = BuildSettings()
        self.build_profiles = BuildProfiles()
        self.scene_validator = SceneValidator(self)
        self.manifest_builder = ManifestBuilder(self)
        self.runtime_exporter = RuntimeExporter(self)
        self.build_report = BuildReport(self)
        self.view_mode = ViewMode(self)

        if self.runtime_mode:
            self.view_mode.mode = ViewMode.GAME_VIEW

        self.runtime_config = RuntimeConfig()
        self.tags_layers_manager = TagsLayersManager(self)

        self.tags = self.tags_layers_manager.tags
        self.layers = self.tags_layers_manager.layers

        self.layer_visibility = LayerVisibility(self)
        self.editor_view_settings = EditorViewSettings(self)
        self.selection_manager = SelectionManager(self)
        self.rts_command_queue = RTSCommandQueue(self)
        self.component_registry = ComponentRegistry()
        self.component_tools = ComponentTools(self)
        self.editor_tools = EditorTools(self)

        self.control_groups = {i: [] for i in range(1, 10)}

        self.autosave_available = self.autosave_manager.autosave_exists()

        self.project_manager.load_project()
        self.load_project_systems()

        if self.runtime_mode:
            start_scene = self.build_settings.get(
                "start_scene",
                self.scene_manager.current_scene
            )
            self.scene_manager.current_scene = start_scene
            self.scene_manager.load_current_scene()

        # =========================
        # UI
        # =========================

        self.menu_bar = MenuBar(self)
        self.toolbar = Toolbar(self)

        self.ui_buttons = []

        self.history = EditorHistory(self)
        self.history.take_snapshot("Initial Scene")

        if self.runtime_mode:
            self.console.log("MiniForge runtime iniciado", "ENGINE")
        else:
            self.console.log(f"✅ {version_label()} iniciado", "ENGINE")
            self.console.log(
                f"Upgrade batch activo: {self.upgrade_manifest.count()} mejoras registradas",
                "ENGINE"
            )
            self.plugin_hook("on_editor_start")

    # =========================
    # PROJECT PATH HELPERS
    # =========================

    def refresh_project_paths(self):
        self.project_path = os.path.normpath(self.project_path)
        self.project_paths = AssetTools.get_project_paths(self.project_path)
        AssetTools.ensure_project_folders(self.project_path)

        if hasattr(self, "asset_database"):
            self.asset_database.root = self.project_asset_path()
            self.asset_database.project_root = self.project_path
            self.asset_database.metadata_file = self.project_join(
                "project",
                "asset_metadata.json"
            )

        if hasattr(self, "resources"):
            self.resources.set_root(self.project_asset_path())

    def project_join(self, *parts):
        return os.path.join(self.project_path, *parts)

    def project_asset_path(self):
        return self.project_paths["assets"]

    def project_sprites_path(self):
        return self.project_paths["sprites"]

    def project_audio_path(self):
        return self.project_paths["audio"]

    def project_data_path(self):
        return self.project_paths["data"]

    def project_prefabs_path(self):
        return self.project_paths["prefabs"]

    def project_scripts_path(self):
        return self.project_paths["scripts"]

    def project_components_path(self):
        return self.project_paths["components"]

    def project_systems_path(self):
        return self.project_paths["systems"]

    def project_scenes_path(self):
        return self.project_paths["scenes"]

    def project_settings_path(self):
        return self.project_paths["settings"]

    def project_logs_path(self):
        return self.project_paths["logs"]

    # =========================
    # SAFE SCANS
    # =========================

    def safe_scan_resources(self):
        try:
            self.resources.scan_all(self.project_path)
        except TypeError:
            try:
                self.resources.scan_all()
            except Exception as error:
                if hasattr(self, "console"):
                    self.console.log(f"Resource scan error: {error}", "ERROR")
        except Exception as error:
            if hasattr(self, "console"):
                self.console.log(f"Resource scan error: {error}", "ERROR")

    def safe_scan_scripts(self):
        try:
            self.script_manager.scan_scripts(
                project_path=self.project_path
            )

            self.console.log(
                f"Scripts escaneados desde proyecto: {self.project_path}",
                "SCRIPT"
            )

        except Exception as error:
            if hasattr(self, "console"):
                self.console.log(f"Script scan error: {error}", "ERROR")

    def hot_reload_scripts(self):
        if self.runtime_mode:
            return

        now = pygame.time.get_ticks() / 1000.0

        if now - self.last_script_reload_check < 1.0:
            return

        self.last_script_reload_check = now

        try:
            changed_resources = self.resources.reload_changed() if hasattr(self.resources, "reload_changed") else 0

            if changed_resources:
                self.console.log(f"Assets recargados: {changed_resources}", "ASSET")

            if self.script_manager.reload_if_changed():
                self.load_project_systems()
                self.console.log("Scripts recargados", "SCRIPT")
        except Exception as error:
            self.console.log(f"Hot reload error: {error}", "WARNING")

    def load_project_systems(self):
        self.project_runtime_systems = []

        if not hasattr(self, "script_manager"):
            return

        systems_root = os.path.normpath(self.project_systems_path())

        for script_name, path in self.script_manager.script_paths.items():
            normalized = os.path.normpath(path)

            try:
                if os.path.commonpath([systems_root, normalized]) != systems_root:
                    continue
            except Exception:
                continue

            cls = self.script_manager.scripts.get(script_name)

            if not cls:
                continue

            try:
                system = cls(self)
            except TypeError:
                try:
                    system = cls()
                    system.game = self
                except Exception as error:
                    self.console.log(
                        f"No se pudo iniciar sistema {script_name}: {error}",
                        "ERROR"
                    )
                    continue
            except Exception as error:
                self.console.log(
                    f"No se pudo iniciar sistema {script_name}: {error}",
                    "ERROR"
                )
                continue

            self.project_runtime_systems.append(system)

        if self.project_runtime_systems:
            self.console.log(
                f"Sistemas de proyecto cargados: {len(self.project_runtime_systems)}",
                "ENGINE"
            )

    # =========================
    # MAIN LOOP
    # =========================

    def run(self):
        while self.running:
            dt = self.clock.tick(
                self.settings.get("target_fps", 60)
            ) / 1000.0

            self.profiler.begin_frame()
            self.system_scheduler.update(dt)

            for system in self.project_runtime_systems:
                if not getattr(system, "enabled", True):
                    continue

                if self.mode == "EDITOR" and not getattr(system, "run_in_editor", False):
                    continue

                if self.mode == "PLAY" and not getattr(system, "run_in_play", True):
                    continue

                self.error_handler.safe_call(
                    f"Project system update {system.__class__.__name__}",
                    system.update,
                    dt
                )

            self.hierarchy_manager.sync_child_world_transforms()
            self.diagnostics.update(dt)
            self.update_profiler_counters()

            self.error_handler.safe_call(
                "RTS command queue",
                self.rts_command_queue.update
            )

            self.error_handler.safe_call(
                "Patrol update",
                self.update_patrol_units
            )

            self.hot_reload_scripts()

            if self.mode == "EDITOR":
                self.error_handler.safe_call(
                    "Autosave update",
                    self.autosave_manager.update
                )

            try:
                self.render_system.draw()

            except Exception as error:
                self.console.log(f"Render crash: {error}", "ERROR")
                self.draw_emergency_error_screen(error)

            self.profiler.end_frame()

        if hasattr(self, "layout_manager"):
            self.layout_manager.save_layout()

        if hasattr(self, "project_manager"):
            self.project_manager.save_project()

    def update_profiler_counters(self):
        if not hasattr(self, "profiler"):
            return

        self.profiler.set_counter("Entities", len(getattr(self, "units", [])))
        self.profiler.set_counter("UI Elements", getattr(getattr(self, "ui_canvas", None), "stats", {}).get("elements", 0))
        self.profiler.set_counter("Visual Graphs", getattr(getattr(self, "visual_script_runtime", None), "stats", {}).get("graphs", 0))
        self.profiler.set_counter("Animators", self.find_component_count("Animator"))
        self.profiler.set_counter("AI Agents", self.find_component_count("AIController"))
        self.profiler.set_counter("Nav Agents", self.find_component_count("NavAgent"))
        self.profiler.set_counter("Gameplay Spawners", self.find_component_count("Spawner"))

    def find_component_count(self, component_type):
        count = 0

        for entity in getattr(self, "units", []):
            if hasattr(entity, "get_component") and entity.get_component(component_type):
                count += 1

        return count

    def draw_emergency_error_screen(self, error):
        try:
            self.screen.fill((22, 24, 30))

            font_big = pygame.font.SysFont(None, 30)
            font = pygame.font.SysFont(None, 22)
            small = pygame.font.SysFont(None, 18)

            title = font_big.render(
                "MiniForge Render Error",
                True,
                (255, 110, 110)
            )

            self.screen.blit(title, (40, 40))

            msg = font.render(
                str(error),
                True,
                (240, 240, 245)
            )

            self.screen.blit(msg, (40, 90))

            hint_lines = [
                "El motor no se cerró, pero el renderer falló.",
                "Revisa projects/DefaultProject/logs/engine.log.",
                "Manda ese error y lo parchamos.",
                "",
                "Teclas útiles:",
                "F1: mostrar/ocultar consola",
                "` : abrir consola",
                "ESC: limpiar selección",
            ]

            y = 135

            for line in hint_lines:
                img = small.render(line, True, (180, 185, 200))
                self.screen.blit(img, (40, y))
                y += 22

            pygame.display.flip()

        except Exception:
            pass

    # =========================
    # CREATE / RENAME MODAL
    # =========================

    def open_create_modal(self, mode):
        titles = {
            "create_script": ("Create Script", "PlayerController"),
            "create_component": ("Create Component", "HealthComponent"),
            "create_system": ("Create System", "WeatherSystem"),
            "create_scene": ("Create Scene", "LevelOne"),
            "create_prefab": ("Create Prefab", "SoldierPrefab"),
            "create_json": ("Create JSON", "BalanceData"),
            "create_txt": ("Create TXT", "Notes"),
            "create_folder": ("Create Folder", "Gameplay"),
            "rename_asset": ("Rename Asset", "NewAssetName"),
            "rename_folder": ("Rename Folder", "NewFolderName"),
        }

        title, placeholder = titles.get(mode, ("Create", "NewAsset"))

        self.create_asset_modal.open(
            mode=mode,
            title=title,
            placeholder=placeholder
        )

    def open_new_folder_modal(self):
        self.open_create_modal("create_folder")

    def open_new_script_modal(self):
        self.open_create_modal("create_script")

    def open_new_component_modal(self):
        self.open_create_modal("create_component")

    def open_new_system_modal(self):
        self.open_create_modal("create_system")

    def open_new_json_modal(self):
        self.open_create_modal("create_json")

    def open_new_txt_modal(self):
        self.open_create_modal("create_txt")

    def open_new_prefab_modal(self):
        self.open_create_modal("create_prefab")

    def open_rename_asset_modal(self):
        self.open_create_modal("rename_asset")

    def open_rename_folder_modal(self):
        self.open_create_modal("rename_folder")

    def create_template_empty(self):
        return self.create_project_template("Empty")

    def create_template_rts(self):
        return self.create_project_template("RTS")

    def create_template_topdown(self):
        return self.create_project_template("TopDown")

    def create_template_platformer(self):
        return self.create_project_template("Platformer")

    def create_template_action_rpg(self):
        return self.create_project_template("ActionRPG")

    def create_template_survival(self):
        return self.create_project_template("Survival")

    def create_project_template(self, template_name):
        created = ProjectTemplates.create(self, template_name)

        if created:
            self.refresh_project()

        return created

    def create_example_action_rpg(self):
        created = self.create_project_template("ActionRPG")

        if created:
            player = self.api.create_game_object("Player", 4, 4)
            player.tag = "Player"
            self.selected_units = [player]
            self.apply_component_preset("TopDown Player")

            enemy = self.api.create_game_object("Enemy", 9, 4)
            enemy.tag = "Enemy"
            self.selected_units = [enemy]
            self.apply_component_preset("Enemy AI")

            npc = self.api.create_game_object("QuestNPC", 6, 7)
            npc.tag = "Neutral"
            self.selected_units = [npc]
            self.apply_component_preset("Quest NPC")
            self.clear_selection()
            self.mark_scene_dirty("Create ActionRPG Example")
            self.history.take_snapshot("Create ActionRPG Example")

        return created

    def create_example_ui_scene(self):
        self.create_ui_label("MiniForge UI", 24, 24)
        self.create_ui_button("Start", 24, 68)
        self.create_ui_progress_bar("Health", 24, 116)
        self.mark_scene_dirty("Create UI Example")
        self.history.take_snapshot("Create UI Example")

    def create_ui_label(self, text="Label", x=24, y=24):
        from engine.component import UIElement

        entity = self.api.create_game_object("UI_Label", 0, 0)
        ui = UIElement("Label")
        ui.text = text
        ui.x = float(x)
        ui.y = float(y)
        ui.width = 220
        ui.height = 32
        ui.text_align = "left"
        entity.add_component(ui)
        self.console.log(f"UI Label creado: {text}", "UI")
        return entity

    def create_ui_button(self, text="Button", x=24, y=64):
        from engine.component import UIElement

        entity = self.api.create_game_object("UI_Button", 0, 0)
        ui = UIElement("Button")
        ui.text = text
        ui.x = float(x)
        ui.y = float(y)
        ui.width = 180
        ui.height = 42
        ui.interactable = True
        entity.add_component(ui)
        self.console.log(f"UI Button creado: {text}", "UI")
        return entity

    def create_ui_progress_bar(self, text="Progress", x=24, y=112):
        from engine.component import UIElement

        entity = self.api.create_game_object("UI_ProgressBar", 0, 0)
        ui = UIElement("ProgressBar")
        ui.text = text
        ui.x = float(x)
        ui.y = float(y)
        ui.width = 240
        ui.height = 24
        ui.progress = 0.75
        ui.max_progress = 1.0
        entity.add_component(ui)
        self.console.log(f"UI ProgressBar creado: {text}", "UI")
        return entity

    def add_visual_script_template(self, template_name="Log And Move"):
        if not self.selected_units:
            self.console.log("Selecciona una entidad primero", "WARNING")
            return None

        entity = self.selected_units[0]
        visual_script = entity.get_component("VisualScript") if hasattr(entity, "get_component") else None

        if not visual_script:
            visual_script = self.component_registry.create("VisualScript")
            entity.add_component(visual_script)

        if template_name == "Button Click":
            visual_script.graph_name = "ButtonClick"
            visual_script.nodes = [
                {"id": "click", "type": "EventClick", "next": "log"},
                {"id": "log", "type": "Log", "message": "Button clicked", "next": None},
            ]
        elif template_name == "Damage Self":
            visual_script.graph_name = "DamageSelf"
            visual_script.nodes = [
                {"id": "start", "type": "EventStart", "next": "damage"},
                {"id": "damage", "type": "Damage", "amount": 5, "next": None},
            ]
        else:
            visual_script.graph_name = "LogAndMove"
            visual_script.nodes = [
                {"id": "start", "type": "EventStart", "next": "log"},
                {"id": "log", "type": "Log", "message": "Visual script running", "next": "move"},
                {"id": "move", "type": "Move", "x": 1, "y": 0, "next": None},
            ]

        self.mark_scene_dirty("Visual Script Template")
        self.history.take_snapshot("Visual Script Template")
        self.console.log(f"VisualScript template aplicado: {visual_script.graph_name}", "SCRIPT")
        return visual_script

    def rebuild_asset_dependency_graph(self, quiet=False):
        if not hasattr(self, "asset_database"):
            return {}

        graph = self.asset_database.rebuild_dependency_graph()
        self.asset_database.save_metadata()

        if not quiet:
            self.console.log(f"Asset dependency graph: {len(graph)} assets", "ASSET")

        return graph

    def print_selected_asset_dependencies(self):
        asset = getattr(self.file_browser, "selected_asset", None)

        if not asset:
            self.console.log("Selecciona un asset primero", "WARNING")
            return []

        relative = asset.get("relative_path")
        dependencies = self.asset_database.dependencies_for(relative)
        reverse = self.asset_database.reverse_dependencies_for(relative)
        self.console.log(f"Deps {relative}: {dependencies}", "ASSET")
        self.console.log(f"Used by {relative}: {reverse}", "ASSET")
        return dependencies

    def cycle_selected_asset_import_setting(self):
        asset = getattr(self.file_browser, "selected_asset", None)

        if not asset:
            self.console.log("Selecciona un asset primero", "WARNING")
            return None

        relative = asset.get("relative_path")
        settings = self.asset_database.get_import_settings(relative)

        if asset.get("type") == "Sprite":
            current = settings.get("filter", "nearest")
            settings = self.asset_database.set_import_setting(
                relative,
                "filter",
                "linear" if current == "nearest" else "nearest",
            )
        elif asset.get("type") == "Audio":
            current = bool(settings.get("stream", False))
            settings = self.asset_database.set_import_setting(relative, "stream", not current)
        else:
            current = bool(settings.get("include_in_build", True))
            settings = self.asset_database.set_import_setting(relative, "include_in_build", not current)

        self.console.log(f"Import settings {relative}: {settings}", "ASSET")
        return settings

    def plugin_hook(self, hook_name):
        if not hasattr(self, "plugin_manager"):
            return 0

        count = self.plugin_manager.emit_hook(hook_name, self)
        self.console.log(f"Plugin hook {hook_name}: {count} handler(s)", "ENGINE")
        return count

    def validate_project(self):
        if hasattr(self, "project_validator"):
            return self.project_validator.validate()

        self.console.log("ProjectValidator no disponible", "WARNING")
        return False

    # =========================
    # NAVIGATOR
    # =========================

    def toggle_navigator_section(self, section_name):
        self.navigator_sections_open[section_name] = not self.navigator_sections_open.get(
            section_name,
            False
        )

    def clear_navigator_search(self):
        self.navigator_search_text = ""
        self.navigator_search_active = False
        self.navigator_scroll = 0

    def get_navigator_actions(self):
        actions = {
            "Scene": [
                ("New Scene", self.new_scene),
                ("Save Scene", self.save_scene),
                ("Load Scene", self.load_scene),
                ("Next Scene", self.next_scene),
                ("Duplicate Scene", self.scene_tools.duplicate_current_scene),
                ("Delete Scene", self.scene_tools.delete_current_scene),
            ],
            "Entity": [
                ("Spawn Entity", self.spawn_unit),
                ("Create GameObject", self.create_game_object),
                ("Duplicate", self.duplicate_selected),
                ("Delete", self.delete_selected),
                ("Parent To Active", self.parent_selection_to_active),
                ("Clear Parent", self.clear_selected_parent),
                ("Create Empty Child", self.create_empty_child),
                ("Duplicate Hierarchy", self.duplicate_selected_with_children),
                ("Delete Hierarchy", self.delete_selected_with_children),
                ("Snap To Grid", self.snap_selected_to_grid),
                ("Align X", self.align_selected_x),
                ("Align Y", self.align_selected_y),
                ("Distribute X", self.distribute_selected_x),
                ("Distribute Y", self.distribute_selected_y),
                ("Center Camera", self.center_camera_on_selection),
                ("Toggle Visible", self.editor_tools.toggle_selected_visible),
                ("Toggle Locked", self.editor_tools.toggle_selected_locked),
            ],
            "Assets": [
                ("Import Sprite", self.import_sprite),
                ("Import Audio", self.import_audio),
                ("Import Data", self.import_data),
                ("New Folder", lambda: self.open_create_modal("create_folder")),
                ("Open Asset", self.file_browser.open_selected),
                ("Duplicate Asset", self.file_browser.duplicate_selected_asset),
                ("Delete Asset", self.file_browser.delete_selected_asset),
                ("Tree View", self.file_browser.toggle_tree_view),
                ("Refresh", self.refresh_project),
            ],
            "Create": [
                ("Script", lambda: self.open_create_modal("create_script")),
                ("Component", lambda: self.open_create_modal("create_component")),
                ("System", lambda: self.open_create_modal("create_system")),
                ("JSON", lambda: self.open_create_modal("create_json")),
                ("TXT", lambda: self.open_create_modal("create_txt")),
                ("Scene File", lambda: self.open_create_modal("create_scene")),
                ("Prefab File", lambda: self.open_create_modal("create_prefab")),
                ("Folder", lambda: self.open_create_modal("create_folder")),
                ("Template ActionRPG", self.create_template_action_rpg),
                ("Template Survival", self.create_template_survival),
                ("Template TopDown", self.create_template_topdown),
                ("Template Platformer", self.create_template_platformer),
                ("Validate Project", self.validate_project),
            ],
            "Scripts": [
                ("New Script", lambda: self.open_create_modal("create_script")),
                ("Add Script", self.add_selected_script),
                ("Script Help", self.toggle_script_help),
            ],
            "Prefabs": [
                ("Save Prefab", self.save_selected_prefab),
                ("Create Empty Prefab", lambda: self.open_create_modal("create_prefab")),
                ("Instantiate", self.instantiate_selected_prefab),
                ("Apply Prefab", self.prefab_workflow.apply_selected_to_prefab),
                ("Revert Prefab", self.prefab_workflow.revert_selected_prefab),
                ("Create Variant", self.create_prefab_variant),
                ("Nested Prefab", self.instantiate_nested_prefab),
            ],
            "Map": [
                ("Next Tile", self.next_tile_brush),
                ("Next Layer", self.next_tilemap_layer),
                ("Layer Visible", self.toggle_tilemap_layer_visible),
                ("Layer Locked", self.toggle_tilemap_layer_locked),
                ("Fill 8x8", self.fill_tilemap_block),
                ("Show Grid", self.editor_tools.toggle_grid),
                ("Show Chunks", self.editor_tools.toggle_chunks),
                ("Show Coords", self.editor_tools.toggle_coordinates),
                ("Brush Preview", self.editor_tools.toggle_brush_preview),
                ("Brush Size +", lambda: self.set_brush_size(self.brush_size + 1)),
                ("Brush Size -", lambda: self.set_brush_size(self.brush_size - 1)),
            ],
            "Components": self.get_component_menu_actions(),
            "UI": [
                ("Create Label", self.create_ui_label),
                ("Create Button", self.create_ui_button),
                ("Create Progress Bar", self.create_ui_progress_bar),
                ("UI Example", self.create_example_ui_scene),
            ],
            "Visual": [
                ("Log And Move", lambda: self.add_visual_script_template("Log And Move")),
                ("Button Click", lambda: self.add_visual_script_template("Button Click")),
                ("Damage Self", lambda: self.add_visual_script_template("Damage Self")),
                ("Visual Input", self.visual_input_editor.toggle),
            ],
            "RTS": [
                ("Stop", self.stop_selected_units),
                ("Hold", self.hold_selected_units),
                ("Cancel", self.command_system.cancel_units),
                ("Formation Square", lambda: self.set_rts_formation("square")),
                ("Formation Line", lambda: self.set_rts_formation("line")),
                ("Formation Column", lambda: self.set_rts_formation("column")),
                ("Formation Circle", lambda: self.set_rts_formation("circle")),
            ],
            "Runtime": [
                ("Play", self.play),
                ("Stop", self.stop),
                ("Pause", self.pause_play_mode),
                ("Restart Play", self.restart_play_mode),
                ("Build & Run", self.build_and_run),
                ("Build Profile", self.cycle_build_profile),
                ("Game / Scene View", self.toggle_view_mode),
                ("Validate Scene", self.validate_scene),
                ("Validate Project", self.validate_project),
                ("Build Manifest", self.build_manifest),
                ("Export Build", self.export_build),
            ],
            "Layout": [
                ("Save Layout", self.save_editor_layout),
                ("Reset Layout", self.reset_editor_layout),
                ("Show Panels", self.show_all_panels),
            ],
            "Plugins": [
                ("Scan Plugins", self.plugin_manager.scan),
                ("Hook Editor Start", lambda: self.plugin_hook("on_editor_start")),
                ("Hook Scene Saved", lambda: self.plugin_hook("on_scene_saved")),
            ],
            "Settings": [
                ("Build Settings", lambda: self.open_settings_panel("Build")),
                ("Build Profiles", lambda: self.open_settings_panel("BuildProfiles")),
                ("Input Settings", lambda: self.open_settings_panel("Input")),
                ("Plugins", lambda: self.open_settings_panel("Plugins")),
                ("Viewport Settings", lambda: self.open_settings_panel("Viewport")),
                ("Tags / Layers", lambda: self.open_settings_panel("TagsLayers")),
                ("Close Settings", self.close_settings_panel),
            ],
        }

        query = self.navigator_search_text.strip().lower()

        if not query:
            return actions

        filtered = {}

        for section, items in actions.items():
            section_match = query in section.lower()

            matching_items = [
                (label, callback)
                for label, callback in items
                if query in label.lower()
            ]

            if section_match:
                filtered[section] = items

            elif matching_items:
                filtered[section] = matching_items

        return filtered

    def get_component_menu_actions(self):
        core_actions = [
            ("SpriteRenderer", self.add_sprite_renderer_component),
            ("RTSMovement", self.add_rts_movement_component),
            ("AudioSource", self.add_audio_source_component),
            ("Rigidbody2D", lambda: self.add_component_to_selected("Rigidbody2D")),
            ("Animator", lambda: self.add_component_to_selected("Animator")),
            ("VisualScript", lambda: self.add_component_to_selected("VisualScript")),
            ("UIElement", lambda: self.add_component_to_selected("UIElement")),
            ("Health", lambda: self.add_component_to_selected("Health")),
            ("Team", lambda: self.add_component_to_selected("Team")),
            ("Worker", lambda: self.add_component_to_selected("Worker")),
            ("ResourceNode", lambda: self.add_component_to_selected("ResourceNode")),
            ("Collider2D", lambda: self.add_component_to_selected("Collider2D")),
        ]

        advanced_names = [
            "Stats",
            "Inventory",
            "Equipment",
            "Ability",
            "AIController",
            "NavAgent",
            "Interaction",
            "Lifetime",
            "Spawner",
            "DamageDealer",
            "CameraFollow",
            "Saveable",
            "Blackboard",
            "StateMachine",
            "QuestLog",
            "Dialogue",
            "Cooldown",
            "StatusEffects",
            "CombatTarget",
            "LootTable",
            "CameraShake",
            "Light2D",
            "ParallaxLayer",
            "TilemapCollider",
            "ObjectiveMarker",
            "Checkpoint",
            "CharacterController2D",
            "EconomyWallet",
            "Timer",
            "Tween",
        ]

        advanced_actions = [
            (name, lambda component_name=name: self.add_component_to_selected(component_name))
            for name in advanced_names
        ]

        utility_actions = [
            ("Copy Component", self.copy_selected_component),
            ("Paste Component", self.paste_selected_component),
            ("Reset Component", self.reset_selected_component),
            ("Preset Playable", lambda: self.apply_component_preset("Playable Unit")),
            ("Preset Platformer Body", lambda: self.apply_component_preset("Platformer Body")),
            ("Preset TopDown Player", lambda: self.apply_component_preset("TopDown Player")),
            ("Preset Platformer Player", lambda: self.apply_component_preset("Platformer Player")),
            ("Preset Enemy AI", lambda: self.apply_component_preset("Enemy AI")),
            ("Preset Quest NPC", lambda: self.apply_component_preset("Quest NPC")),
            ("Preset Pickup Item", lambda: self.apply_component_preset("Pickup Item")),
            ("Preset Projectile", lambda: self.apply_component_preset("Combat Projectile")),
            ("Preset Spawner", lambda: self.apply_component_preset("Spawner Enemy")),
            ("Preset Checkpoint", lambda: self.apply_component_preset("Checkpoint")),
            ("Preset Door", lambda: self.apply_component_preset("Interactable Door")),
        ]

        return core_actions + advanced_actions + utility_actions

    # =========================
    # CREATE PROJECT FILES
    # =========================

    def create_project_script(self):
        path = self.file_browser.create_script()
        self.refresh_project()

        if path:
            self.console.log(f"Script listo: {path}", "SCRIPT")

    def create_project_component(self):
        path = self.file_browser.create_component()
        self.refresh_project()

        if path:
            self.console.log(f"Componente externo listo: {path}", "ENGINE")

    def create_project_system(self):
        path = self.file_browser.create_system()
        self.refresh_project()

        if path:
            self.console.log(f"Sistema externo listo: {path}", "ENGINE")

    def create_project_json(self):
        path = self.file_browser.create_json()
        self.refresh_project()

        if path:
            self.console.log(f"JSON listo: {path}", "ASSET")

    def create_project_txt(self):
        path = self.file_browser.create_txt()
        self.refresh_project()

        if path:
            self.console.log(f"TXT listo: {path}", "ASSET")

    def create_project_scene_file(self):
        path = self.file_browser.create_scene()
        self.refresh_project()

        if path:
            self.console.log(f"Scene file listo: {path}", "SCENE")

    def create_project_prefab_file(self):
        path = self.file_browser.create_prefab()
        self.refresh_project()

        if path:
            self.console.log(f"Prefab file listo: {path}", "ASSET")

    def create_special_folder(self, folder_type):
        path = self.file_browser.create_special_folder(folder_type)
        self.refresh_project()

        if path:
            self.console.log(f"Folder listo: {path}", "ASSET")

    # =========================
    # SETTINGS PANELS
    # =========================

    def open_settings_panel(self, panel_name):
        self.active_settings_panel = panel_name
        self.settings_editing_key = None
        self.settings_edit_buffer = ""

        self.console.log(f"Settings panel: {panel_name}", "EDITOR")

    def close_settings_panel(self):
        self.active_settings_panel = None
        self.settings_editing_key = None
        self.settings_edit_buffer = ""

    def start_settings_edit(self, key, current_value):
        self.settings_editing_key = key
        self.settings_edit_buffer = str(current_value)

    def cancel_settings_edit(self):
        self.settings_editing_key = None
        self.settings_edit_buffer = ""

    def commit_settings_edit(self):
        key = self.settings_editing_key

        if not key:
            return

        value = self.settings_edit_buffer.strip()

        if value.lower() in ["true", "false"]:
            parsed_value = value.lower() == "true"

        else:
            try:
                if "." in value:
                    parsed_value = float(value)
                else:
                    parsed_value = int(value)

            except Exception:
                parsed_value = value

        if self.active_settings_panel == "Build":
            self.build_settings.set(key, parsed_value)
            self.console.log(f"Build setting {key} = {parsed_value}", "ENGINE")

        elif self.active_settings_panel == "Viewport":
            self.editor_view_settings.set(key, parsed_value)
            self.console.log(f"Viewport setting {key} = {parsed_value}", "EDITOR")

        elif self.active_settings_panel == "Input":
            keys = [
                item.strip()
                for item in str(parsed_value).split(",")
                if item.strip()
            ]
            self.input_map.set_binding(key, keys)
            self.console.log(f"Input {key} = {keys}", "ENGINE")

        self.cancel_settings_edit()

    def toggle_viewport_setting(self, key):
        self.editor_view_settings.toggle(key)

    def get_build_settings_rows(self):
        return list(self.build_settings.data.items())

    def get_viewport_settings_rows(self):
        return list(self.editor_view_settings.data.items())

    def get_input_settings_rows(self):
        return list(self.input_map.bindings.items())

    def get_build_profile_rows(self):
        return [
            ("active", self.build_profiles.active),
            *[
                (name, profile)
                for name, profile in self.build_profiles.profiles.items()
            ],
        ]

    def get_plugin_rows(self):
        self.plugin_manager.scan()
        return [
            (plugin.get("name", "Plugin"), plugin.get("path", ""))
            for plugin in self.plugin_manager.plugins
        ]

    def cycle_build_profile(self):
        name = self.build_profiles.cycle()
        self.build_profiles.apply_to(self.build_settings)
        self.console.log(f"Build profile activo: {name}", "BUILD")
        return name

    def add_tag_from_panel(self):
        name = f"Tag_{len(self.tags) + 1}"
        self.tags_layers_manager.add_tag(name)
        self.tags = self.tags_layers_manager.tags

    def add_layer_from_panel(self):
        name = f"Layer_{len(self.layers) + 1}"
        self.tags_layers_manager.add_layer(name)
        self.layers = self.tags_layers_manager.layers
        self.layer_visibility.sync_layers()

    def cycle_selected_tag_from_panel(self):
        self.cycle_tag_selected()

    def cycle_selected_layer_from_panel(self):
        self.cycle_layer_selected()

    # =========================
    # INSPECTOR SECTIONS
    # =========================

    def toggle_inspector_section(self, section_name):
        self.inspector_sections_open[section_name] = not self.inspector_sections_open.get(
            section_name,
            True
        )

    def inspector_quick_action(self, action):
        if action == "reset_transform":
            self.reset_selected_transform()

        elif action == "clear_path":
            self.clear_selected_paths()

        elif action == "toggle_enabled":
            self.toggle_selected_enabled()

        elif action == "toggle_visible":
            self.editor_tools.toggle_selected_visible()

        elif action == "toggle_locked":
            self.editor_tools.toggle_selected_locked()

        elif action == "clear_scripts":
            self.clear_scripts_from_selected()

        elif action == "add_selected_component":
            asset = self.file_browser.selected_asset

            if asset and asset.get("type") == "Component":
                self.add_component_to_selected(asset.get("name"))
            else:
                self.console.log(
                    "Selecciona un componente en el Content Browser",
                    "WARNING"
                )

        elif action == "copy_component":
            self.copy_selected_component()

        elif action == "paste_component":
            self.paste_selected_component()

        elif action == "reset_component":
            self.reset_selected_component()

        elif action == "preset_playable":
            self.apply_component_preset("Playable Unit")

        elif action == "preset_topdown":
            self.apply_component_preset("TopDown Player")

        elif action == "preset_enemy":
            self.apply_component_preset("Enemy AI")

        elif action == "preset_npc":
            self.apply_component_preset("Quest NPC")

        elif action == "preset_projectile":
            self.apply_component_preset("Combat Projectile")

        elif action == "apply_prefab":
            self.prefab_workflow.apply_selected_to_prefab()

        elif action == "revert_prefab":
            self.prefab_workflow.revert_selected_prefab()

    # =========================
    # SCENES
    # =========================

    def new_scene(self):
        self.scene_manager.create_new_scene()
        self.refresh_project()

    def save_scene(self):
        if self.mode == "PLAY":
            self.console.log(
                "No puedes guardar en PLAY MODE. Vuelve a EDITOR.",
                "WARNING"
            )
            return

        valid = self.scene_validator.validate()

        if not valid:
            self.console.log(
                "Escena tiene errores. Guardado cancelado.",
                "ERROR"
            )
            return

        if hasattr(self, "scene_tools"):
            self.scene_tools.backup_current_scene()

        self.scene_manager.save_current_scene()
        self.mark_scene_clean()
        self.manifest_builder.build_manifest()
        self.refresh_project()
        self.plugin_hook("on_scene_saved")

    def load_scene(self):
        self.scene_manager.load_current_scene()
        self.resolve_entity_asset_references()
        self.mark_scene_clean()

    def next_scene(self):
        self.scene_manager.next_scene()

    # =========================
    # PROJECT / AUTOSAVE
    # =========================

    def save_project(self):
        self.project_manager.save_project()

    def autosave_now(self):
        self.autosave_manager.save()
        self.autosave_available = True
        self.console.log("Autosave creado", "ENGINE")

    def save_autosave(self):
        self.autosave_now()

    def load_autosave(self):
        self.autosave_manager.load_autosave()

    def recover_autosave(self):
        if not self.autosave_manager.autosave_exists():
            self.console.log("No hay autosave disponible", "WARNING")
            return

        self.autosave_manager.load_autosave()
        self.autosave_available = False
        self.console.log("Autosave recuperado", "ENGINE")

    # =========================
    # MODE / VIEW
    # =========================

    def toggle_mode(self):
        if self.mode == "EDITOR":
            valid = self.scene_validator.validate()

            if not valid:
                self.console.log(
                    "No se puede entrar a PLAY: hay errores en escena",
                    "ERROR"
                )
                return

        self.play_mode_manager.toggle()

    def play(self):
        if self.mode != "PLAY":
            self.toggle_mode()

    def stop(self):
        if self.mode == "PLAY":
            self.play_mode_manager.exit_play_mode()

    def pause_play_mode(self):
        return self.play_mode_manager.toggle_pause()

    def restart_play_mode(self):
        return self.play_mode_manager.restart()

    def toggle_view_mode(self):
        self.view_mode.toggle()

    def cycle_editor_tab(self):
        tab = self.editor_tabs.cycle()
        self.active_editor_tab = tab
        self.console.log(f"Editor tab: {tab}", "EDITOR")
        return tab

    def toggle_console(self):
        self.console.toggle()

    def cycle_console_filter(self):
        self.console.cycle_filter()

    def ui_captures_keyboard(self):
        if getattr(self, "runtime_mode", False):
            return False

        checks = [
            getattr(self.console, "input_active", False),
            getattr(self.script_editor, "visible", False),
            getattr(self.command_palette, "visible", False),
            getattr(self.visual_input_editor, "visible", False),
            getattr(self.create_asset_modal, "visible", False),
            getattr(self, "navigator_search_active", False),
            getattr(self, "settings_editing_key", None) is not None,
            getattr(self.scene_hierarchy, "search_active", False),
            getattr(self.inspector_editor, "editing", False),
        ]

        return any(checks)

    def save_editor_layout(self):
        self.layout_manager.save_layout()

    def reset_editor_layout(self):
        self.layout_manager.reset_layout()

    def show_all_panels(self):
        self.layout_manager.show_all_panels()

    def validate_scene(self):
        self.scene_validator.validate()

    def build_manifest(self):
        self.manifest_builder.build_manifest()

    def export_build(self):
        if self.mode == "PLAY" and not self.runtime_mode:
            self.console.log("Vuelve a EDITOR antes de exportar.", "WARNING")
            return None

        if not self.validate_project():
            self.console.log("Build cancelado: corrige errores del proyecto.", "ERROR")
            return None

        self.build_profiles.apply_to(self.build_settings)
        self.save_scene()
        self.build_manifest()
        self.asset_database.scan()
        build_path = self.runtime_exporter.export()

        if build_path:
            self.build_report.generate(build_path)

        return build_path

    def build_and_run(self):
        build_path = self.export_build()

        if not build_path:
            return None

        runner = os.path.join(build_path, "run_game.py")

        if not os.path.exists(runner):
            self.console.log("Build creado, pero falta run_game.py", "ERROR")
            return None

        try:
            subprocess.Popen([sys.executable, runner], cwd=build_path)
            self.console.log(f"Build ejecutado: {build_path}", "BUILD")
        except Exception as error:
            self.console.log(f"No se pudo ejecutar build: {error}", "ERROR")

        return build_path

    def center_camera_on_selection(self):
        if not self.selected_units:
            self.console.log(
                "No hay selección para centrar cámara",
                "WARNING"
            )
            return

        unit = self.selected_units[0]
        world_x = unit.x * self.grid.tile_size
        world_y = unit.y * self.grid.tile_size

        self.center_camera_on_world(world_x, world_y)

    # =========================
    # TOOLS / MAP
    # =========================

    def set_tool(self, tool_name):
        self.active_tool = tool_name
        self.console.log(f"Herramienta activa: {tool_name}", "ENGINE")

    def set_brush_size(self, size):
        self.brush_size = max(1, min(5, int(size)))
        self.console.log(f"Brush size: {self.brush_size}", "EDITOR")

    def next_tile_brush(self):
        self.tile_brush += 1

        if self.tile_brush > 4:
            self.tile_brush = 0

        self.console.log(f"Tile brush: {self.tile_brush_name()}", "ENGINE")

    def tile_brush_name(self):
        names = getattr(self.grid, "TILE_NAMES", None)

        if names:
            return names.get(self.tile_brush, "Unknown")

        fallback = {
            0: "Grass",
            1: "Obstacle",
            2: "Sand",
            3: "Water",
            4: "Stone",
        }

        return fallback.get(self.tile_brush, "Unknown")

    def screen_to_grid(self, screen_pos):
        mouse_x, mouse_y = screen_pos
        world_x, world_y = self.camera.screen_to_world(mouse_x, mouse_y)

        grid_x = int(world_x // self.grid.tile_size)
        grid_y = int(world_y // self.grid.tile_size)

        return grid_x, grid_y

    def paint_tile_at_screen(self, screen_pos):
        grid_x, grid_y = self.screen_to_grid(screen_pos)
        self.paint_tile_area(grid_x, grid_y, self.tile_brush)

    def paint_obstacle_at_screen(self, screen_pos, value):
        grid_x, grid_y = self.screen_to_grid(screen_pos)
        self.paint_tile_area(grid_x, grid_y, value)

    def paint_tile_area(self, center_x, center_y, value):
        radius = max(1, int(getattr(self, "brush_size", 1)))

        for y in range(center_y - radius + 1, center_y + radius):
            for x in range(center_x - radius + 1, center_x + radius):
                if self.grid.is_inside(x, y):
                    self.grid.set_tile(x, y, value)

                    if hasattr(self, "tilemap_layers"):
                        if value == 0 and self.active_tool == "Erase":
                            self.tilemap_layers.erase_tile(x, y)
                        else:
                            self.tilemap_layers.set_tile(x, y, value)

        if hasattr(self, "mark_scene_dirty"):
            self.mark_scene_dirty("Paint Tilemap")

    def place_entity_at_screen(self, screen_pos):
        self.place_entity_or_prefab_at_screen(screen_pos)

    # =========================
    # IMPORTS / ASSETS
    # =========================

    def import_sprite(self):
        if AssetTools.import_sprite(self.console, self.project_path):
            self.refresh_project()
            self.plugin_hook("on_asset_imported")

    def import_audio(self):
        if AssetTools.import_audio(self.console, self.project_path):
            self.refresh_project()
            self.plugin_hook("on_asset_imported")

    def import_data(self):
        if AssetTools.import_data(self.console, self.project_path):
            self.refresh_project()
            self.plugin_hook("on_asset_imported")

    def refresh_project(self):
        self.refresh_project_paths()

        try:
            self.asset_database.scan()
        except Exception as error:
            self.console.log(
                f"Asset database scan error: {error}",
                "WARNING"
            )

        self.safe_scan_resources()
        self.safe_scan_scripts()

        try:
            self.project_browser.refresh()
        except Exception:
            pass

        try:
            self.file_browser.refresh()
        except Exception as error:
            self.console.log(
                f"File browser refresh error: {error}",
                "WARNING"
            )

        if hasattr(self, "scene_manager"):
            try:
                self.scene_manager.refresh()
            except Exception:
                pass

        self.console.log("Proyecto actualizado", "ASSET")
        self.rebuild_asset_dependency_graph(quiet=True)

    def mark_scene_dirty(self, reason="Change"):
        self.scene_dirty = True
        self.scene_dirty_reason = reason

    def mark_scene_clean(self):
        self.scene_dirty = False
        self.scene_dirty_reason = ""

    def resolve_entity_asset_references(self):
        for entity in self.units:
            sprite_renderer = (
                entity.get_component("SpriteRenderer")
                if hasattr(entity, "get_component")
                else None
            )

            if sprite_renderer:
                sprite_renderer.sprite_name = self.asset_resolver.sprite_name(
                    getattr(sprite_renderer, "sprite_guid", None),
                    getattr(sprite_renderer, "sprite_name", None)
                )
                entity.sprite_name = sprite_renderer.sprite_name

    # =========================
    # PREFABS
    # =========================

    def save_selected_prefab(self):
        if not self.selected_units:
            self.console.log("Selecciona una entidad primero", "WARNING")
            return

        unit = self.selected_units[0]
        filename = f"{getattr(unit, 'name', 'unit').lower()}_prefab.prefab"

        path = PrefabManager.save_prefab(unit, filename)

        self.refresh_project()
        self.console.log(f"Prefab guardado: {path}", "ASSET")

    def instantiate_selected_prefab(self):
        asset = self.file_browser.selected_asset

        if not asset or asset["type"] != "Prefab":
            self.console.log(
                "Selecciona un prefab en el Content Browser",
                "WARNING"
            )
            return False

        mouse_x, mouse_y = pygame.mouse.get_pos()
        grid_x, grid_y = self.screen_to_grid((mouse_x, mouse_y))

        unit = PrefabManager.instantiate_prefab(
            self,
            asset["path"],
            grid_x,
            grid_y
        )

        if unit:
            self.history.take_snapshot("Instantiate Prefab")
            self.console.log("Prefab instanciado", "ASSET")
            return True

        return False

    def instantiate_prefab_at_grid(self, asset, grid_x, grid_y):
        if not asset or asset["type"] != "Prefab":
            return None

        unit = PrefabManager.instantiate_prefab(
            self,
            asset["path"],
            grid_x,
            grid_y
        )

        if unit:
            self.history.take_snapshot("Place Prefab")
            self.console.log(
                f"Prefab colocado en {grid_x}, {grid_y}",
                "ASSET"
            )

        return unit

    # =========================
    # SCRIPTING
    # =========================

    def create_script(self):
        self.open_create_modal("create_script")

    def toggle_script_help(self):
        self.script_editor.toggle_help()

    def add_selected_script(self):
        if not self.selected_units:
            self.console.log("Selecciona una entidad primero", "WARNING")
            return

        asset = self.file_browser.selected_asset

        if not asset or asset["type"] not in ["Script", "Component", "System"]:
            self.console.log(
                "Selecciona un script en el Content Browser",
                "WARNING"
            )
            return

        script_name = asset["name"]
        script = self.script_manager.create(script_name)

        if not script:
            self.console.log(
                f"No se pudo crear script: {script_name}",
                "ERROR"
            )
            return

        self.selected_units[0].add_script(script)
        self.history.take_snapshot("Add Script")
        self.mark_scene_dirty("Add Script")
        self.console.log(f"Script añadido: {script_name}", "SCRIPT")

    # =========================
    # COMPONENTS
    # =========================

    def add_component_to_selected(self, component_name):
        if not hasattr(self, "component_registry"):
            self.console.log("ComponentRegistry no activo", "WARNING")
            return

        if not self.selected_units:
            self.console.log("Selecciona una entidad primero", "WARNING")
            return

        added = 0

        for unit in self.selected_units:
            component = self.component_registry.create(component_name)

            if component:
                unit.add_component(component)
                self.repair_entity_components(unit)
                added += 1

        self.history.take_snapshot(f"Add Component {component_name}")
        self.mark_scene_dirty(f"Add Component {component_name}")

        self.console.log(
            f"{component_name} agregado a {added} entidad(es)",
            "ENGINE"
        )

    def repair_entity_components(self, entity):
        from engine.component_validation import ComponentValidation

        repaired = 0

        for component in getattr(entity, "components", []):
            if ComponentValidation.repair_component(component):
                repaired += 1

        if repaired:
            if hasattr(entity, "sync_from_components"):
                entity.sync_from_components()

            if hasattr(entity, "sync_to_components"):
                entity.sync_to_components()

        return repaired

    def add_sprite_renderer_component(self):
        self.add_component_to_selected("SpriteRenderer")

    def add_rts_movement_component(self):
        self.add_component_to_selected("RTSMovement")

    def add_audio_source_component(self):
        self.add_component_to_selected("AudioSource")

    def copy_selected_component(self):
        return self.component_tools.copy()

    def paste_selected_component(self):
        return self.component_tools.paste()

    def reset_selected_component(self):
        return self.component_tools.reset()

    def apply_component_preset(self, preset_name):
        return self.component_tools.apply_preset(preset_name)

    def create_prefab_variant(self):
        return self.advanced_prefabs.create_variant_from_selected()

    def instantiate_nested_prefab(self):
        asset = getattr(self.file_browser, "selected_asset", None)
        path = asset.get("path") if asset and asset.get("type") == "Prefab" else None
        return self.advanced_prefabs.instantiate_nested_prefab_as_child(path)

    def toggle_profiler_pause(self):
        paused = self.profiler.toggle_pause()
        self.console.log(f"Profiler {'pausado' if paused else 'activo'}", "ENGINE")
        return paused

    def next_tilemap_layer(self):
        layer = self.tilemap_layers.cycle_layer(1)

        if layer:
            self.console.log(f"Tile layer activa: {layer.name}", "MAP")
            self.mark_scene_dirty("Tilemap Layer")

        return layer

    def toggle_tilemap_layer_visible(self):
        visible = self.tilemap_layers.toggle_active_visible()
        layer = self.tilemap_layers.active_layer
        self.console.log(f"Tile layer {layer.name} visible={visible}", "MAP")
        self.mark_scene_dirty("Tilemap Layer Visible")
        return visible

    def toggle_tilemap_layer_locked(self):
        locked = self.tilemap_layers.toggle_active_locked()
        layer = self.tilemap_layers.active_layer
        self.console.log(f"Tile layer {layer.name} locked={locked}", "MAP")
        self.mark_scene_dirty("Tilemap Layer Locked")
        return locked

    def fill_tilemap_block(self):
        layer = self.tilemap_layers.active_layer

        if not layer:
            return 0

        x = int(self.camera.x // self.grid.tile_size)
        y = int(self.camera.y // self.grid.tile_size)
        changed = self.tilemap_layers.fill_active(x, y, 8, 8, self.tile_brush)
        self.console.log(f"{changed} tiles pintados en {layer.name}", "MAP")
        self.mark_scene_dirty("Tilemap Fill")
        return changed

    def cycle_tag_selected(self):
        if not self.selected_units:
            self.console.log("Selecciona una entidad primero", "WARNING")
            return

        for unit in self.selected_units:
            unit.tag = self.tags_layers_manager.cycle_tag(
                getattr(unit, "tag", "Untagged")
            )

        self.history.take_snapshot("Cycle Tag")
        self.console.log(f"Tag: {self.selected_units[0].tag}", "ENGINE")

    def cycle_layer_selected(self):
        if not self.selected_units:
            self.console.log("Selecciona una entidad primero", "WARNING")
            return

        for unit in self.selected_units:
            unit.layer = self.tags_layers_manager.cycle_layer(
                getattr(unit, "layer", "Default")
            )

        self.history.take_snapshot("Cycle Layer")
        self.console.log(f"Layer: {self.selected_units[0].layer}", "ENGINE")

    def assign_selected_sprite(self):
        if not self.selected_units:
            self.console.log("Selecciona una entidad primero", "WARNING")
            return

        asset = self.file_browser.selected_asset

        if not asset or asset["type"] != "Sprite":
            self.console.log(
                "Selecciona un sprite en el Content Browser",
                "WARNING"
            )
            return

        for unit in self.selected_units:
            self.asset_resolver.attach_sprite_reference(unit, asset)

        self.history.take_snapshot("Assign Sprite")
        self.mark_scene_dirty("Assign Sprite")
        self.console.log(f"Sprite asignado: {asset['name']}", "ASSET")

    def parent_selection_to_active(self):
        if len(self.selected_units) < 2:
            self.console.log("Selecciona hijos y luego el padre activo", "WARNING")
            return

        parent = self.selected_units[-1]

        for child in self.selected_units[:-1]:
            self.hierarchy_manager.set_parent(child, parent, keep_world=True)

        self.history.take_snapshot("Parent Entities")
        self.console.log(f"Parent asignado: {parent.name}", "EDITOR")

    def clear_selected_parent(self):
        if not self.selected_units:
            self.console.log("Selecciona una entidad primero", "WARNING")
            return

        for entity in self.selected_units:
            self.hierarchy_manager.clear_parent(entity)

        self.history.take_snapshot("Clear Parent")
        self.console.log("Parent eliminado", "EDITOR")

    def create_empty_child(self):
        return self.scene_view_tools.create_empty_child()

    def duplicate_selected_with_children(self):
        return self.scene_view_tools.duplicate_selected_with_children()

    def delete_selected_with_children(self):
        return self.scene_view_tools.delete_selected_with_children()

    def align_selected_x(self):
        self.scene_view_tools.align_selected("x")

    def align_selected_y(self):
        self.scene_view_tools.align_selected("y")

    def distribute_selected_x(self):
        self.scene_view_tools.distribute_selected("x")

    def distribute_selected_y(self):
        self.scene_view_tools.distribute_selected("y")

    def snap_selected_to_grid(self):
        self.scene_view_tools.snap_selected()

    def cycle_gizmo_mode(self):
        return self.scene_view_tools.cycle_gizmo_mode()

    def toggle_grid_snapping(self):
        self.scene_view_tools.toggle_snapping()

    # =========================
    # ENTITIES
    # =========================

    def spawn_unit(self):
        mouse_x, mouse_y = pygame.mouse.get_pos()
        grid_x, grid_y = self.screen_to_grid((mouse_x, mouse_y))

        self.spawn_unit_at_grid(grid_x, grid_y)

    def create_game_object(self):
        mouse_x, mouse_y = pygame.mouse.get_pos()
        grid_x, grid_y = self.screen_to_grid((mouse_x, mouse_y))

        if not self.grid.is_inside(grid_x, grid_y):
            grid_x, grid_y = 0, 0

        obj = GameObject(grid_x, grid_y, self, name="GameObject")
        self.units.append(obj)
        self.world.entities = self.units

        self.clear_selection()
        self.add_to_selection(obj)
        self.mark_scene_dirty("Create GameObject")
        self.history.take_snapshot("Create GameObject")
        self.console.log(f"GameObject creado en {grid_x}, {grid_y}", "ENGINE")
        return obj

    def spawn_unit_at_grid(self, grid_x, grid_y):
        if not self.grid.is_inside(grid_x, grid_y):
            grid_x = max(0, min(self.grid.width - 1, grid_x))
            grid_y = max(0, min(self.grid.height - 1, grid_y))

        if not self.grid.is_walkable(grid_x, grid_y):
            grid_x, grid_y = self.grid.nearest_walkable(grid_x, grid_y)

        unit = Unit(grid_x, grid_y, self)

        self.units.append(unit)
        self.world.entities = self.units

        self.mark_scene_dirty("Spawn Entity")
        self.history.take_snapshot("Spawn Entity")

        self.console.log(
            f"Entidad colocada en {grid_x}, {grid_y}",
            "ENGINE"
        )

    def place_entity_or_prefab_at_screen(self, screen_pos):
        grid_x, grid_y = self.screen_to_grid(screen_pos)
        asset = self.file_browser.selected_asset

        if asset and asset["type"] == "Prefab":
            unit = self.instantiate_prefab_at_grid(asset, grid_x, grid_y)

            if unit:
                return

        self.spawn_unit_at_grid(grid_x, grid_y)

    def duplicate_selected(self):
        if not self.selected_units:
            self.console.log("No hay entidad seleccionada", "WARNING")
            return

        new_units = []

        for unit in self.selected_units:
            duplicate = PrefabManager.entity_from_data(
                self,
                unit.serialize(),
                preserve_id=False
            )

            if not duplicate:
                continue

            duplicate.x += 1
            duplicate.y += 1
            duplicate.sync_to_components()

            self.units.append(duplicate)
            new_units.append(duplicate)

        self.clear_selection()

        for unit in new_units:
            self.add_to_selection(unit)

        self.world.entities = self.units

        self.history.take_snapshot("Duplicate")
        self.mark_scene_dirty("Duplicate")
        self.console.log("Entidad duplicada", "ENGINE")

    def delete_selected(self):
        if not self.selected_units:
            self.console.log("No hay entidad seleccionada", "WARNING")
            return

        for unit in list(self.selected_units):
            if unit in self.units:
                self.units.remove(unit)

        self.selected_units.clear()
        self.world.entities = self.units

        self.history.take_snapshot("Delete")
        self.mark_scene_dirty("Delete")
        self.console.log("Entidad eliminada", "ENGINE")

    def get_entity_by_id(self, entity_id):
        for unit in self.units:
            if getattr(unit, "id", None) == entity_id:
                return unit

        return None

    def rename_selected_entity(self, new_name):
        if not self.selected_units:
            self.console.log("No hay entidad seleccionada", "WARNING")
            return

        self.selected_units[0].name = new_name

        self.history.take_snapshot("Rename Entity")
        self.console.log(f"Entidad renombrada: {new_name}", "ENGINE")

    # =========================
    # CONTROL GROUPS
    # =========================

    def assign_control_group(self, group_number):
        if group_number not in self.control_groups:
            return

        self.control_groups[group_number] = [
            unit.id for unit in self.selected_units
        ]

        self.console.log(
            f"Grupo {group_number} guardado con {len(self.selected_units)} entidad(es)",
            "RTS"
        )

    def select_control_group(self, group_number):
        if group_number not in self.control_groups:
            return False

        ids = self.control_groups[group_number]

        if not ids:
            return False

        self.clear_selection()

        for entity_id in ids:
            unit = self.get_entity_by_id(entity_id)

            if unit:
                self.add_to_selection(unit)

        self.console.log(
            f"Grupo {group_number} seleccionado: {len(self.selected_units)} entidad(es)",
            "RTS"
        )

        return True

    def serialize_control_groups(self):
        return {
            str(group): ids
            for group, ids in self.control_groups.items()
        }

    def deserialize_control_groups(self, data):
        for key, ids in data.items():
            try:
                group = int(key)

                if group in self.control_groups:
                    self.control_groups[group] = ids

            except Exception:
                pass

    # =========================
    # RTS ORDERS
    # =========================

    def set_rts_formation(self, formation):
        self.command_system.default_formation = formation
        self.console.log(f"Formation: {formation}", "RTS")

    def stop_selected_units(self):
        self.command_system.stop_units()
        self.history.take_snapshot("Stop Units")

    def hold_selected_units(self):
        self.command_system.hold_position()
        self.history.take_snapshot("Hold Units")

    def patrol_selected_units_to_screen(self, screen_pos):
        grid_x, grid_y = self.screen_to_grid(screen_pos)
        self.command_system.patrol_units((grid_x, grid_y))
        self.history.take_snapshot("Patrol Units")

    def update_patrol_units(self):
        for unit in self.units:
            if getattr(unit, "command", None) != "PATROL":
                continue

            if getattr(unit, "path", []):
                continue

            points = getattr(unit, "patrol_points", [])

            if len(points) < 2:
                continue

            current_index = getattr(unit, "patrol_index", 0)
            next_index = 1 - current_index

            unit.patrol_index = next_index
            next_point = points[next_index]

            self.command_system.move_specific_unit_to(unit, next_point)

    # =========================
    # EDITOR ACTIONS
    # =========================

    def reset_selected_transform(self):
        if not self.selected_units:
            self.console.log("No hay entidad seleccionada", "WARNING")
            return

        for unit in self.selected_units:
            unit.x = 0
            unit.y = 0
            unit.path = []
            unit.sync_to_components()

        self.history.take_snapshot("Reset Transform")
        self.console.log("Transform reseteado", "ENGINE")

    def clear_selected_paths(self):
        for unit in self.selected_units:
            unit.path = []
            unit.command = "IDLE"
            unit.state = "IDLE"

        self.history.take_snapshot("Clear Paths")
        self.console.log("Rutas limpiadas", "RTS")

    def toggle_selected_enabled(self):
        for unit in self.selected_units:
            unit.enabled = not getattr(unit, "enabled", True)

        self.history.take_snapshot("Toggle Enabled")
        self.console.log("Enabled cambiado", "ENGINE")

    def remove_component_from_selected(self, component_type):
        if not self.selected_units:
            self.console.log("No hay entidad seleccionada", "WARNING")
            return

        for unit in self.selected_units:
            unit.remove_component(component_type)

        self.history.take_snapshot(f"Remove {component_type}")
        self.console.log(f"Componente eliminado: {component_type}", "ENGINE")

    def clear_scripts_from_selected(self):
        for unit in self.selected_units:
            unit.scripts.clear()

        self.history.take_snapshot("Clear Scripts")
        self.console.log("Scripts eliminados de entidad", "SCRIPT")

    def center_camera_on_world(self, world_x, world_y):
        viewport = self.get_world_viewport_rect()
        self.camera.x = world_x - viewport.width / 2 / self.camera.zoom
        self.camera.y = world_y - viewport.height / 2 / self.camera.zoom
        self.camera.clamp()

    # =========================
    # HISTORY
    # =========================

    def undo(self):
        self.history.undo()

    def redo(self):
        self.history.redo()

    # =========================
    # SELECTION
    # =========================

    def clear_selection(self):
        if hasattr(self, "selection_manager"):
            self.selection_manager.clear()
            return

        for unit in self.units:
            unit.set_selected(False)

        self.selected_units = []

    def add_to_selection(self, unit):
        if hasattr(self, "selection_manager"):
            self.selection_manager.add(unit)
            return

        if not unit or unit in self.selected_units:
            return

        unit.set_selected(True)
        self.selected_units.append(unit)

    def remove_from_selection(self, unit):
        if hasattr(self, "selection_manager"):
            self.selection_manager.remove(unit)
            return

        if unit in self.selected_units:
            unit.set_selected(False)
            self.selected_units.remove(unit)

    def select_at_screen(self, screen_x, screen_y, shift=False):
        if hasattr(self, "selection_manager"):
            self.selection_manager.select_at_screen(screen_x, screen_y, shift)
            return

    def select_in_box(self, x1, y1, x2, y2, shift=False, contains=False):
        if hasattr(self, "selection_manager"):
            self.selection_manager.select_in_box(x1, y1, x2, y2, shift, contains)
            return

    def get_unit_screen_rect(self, unit):
        tile = self.grid.tile_size

        world_x = unit.x * tile
        world_y = unit.y * tile

        screen_x, screen_y = self.camera.world_to_screen(world_x, world_y)

        scale_x = 1.0
        scale_y = 1.0

        if hasattr(unit, "get_component"):
            transform = unit.get_component("Transform")

            if transform:
                scale_x = max(0.05, getattr(transform, "scale_x", 1.0))
                scale_y = max(0.05, getattr(transform, "scale_y", 1.0))

        size = int(tile * self.camera.zoom * 0.75)
        size = max(12, size)

        rect = pygame.Rect(
            int(screen_x),
            int(screen_y),
            max(8, int(size * scale_x)),
            max(8, int(size * scale_y))
        )

        return rect.inflate(8, 8)

    def get_world_viewport_rect(self):
        width, height = self.screen.get_size()

        if getattr(self, "runtime_mode", False):
            return pygame.Rect(0, 0, width, height)

        if hasattr(self, "view_mode") and self.view_mode.is_game_view():
            return pygame.Rect(0, 0, width, height - 90)

        if getattr(self, "mode", "EDITOR") == "PLAY":
            return pygame.Rect(205, 64, width - 205, height - 194)

        return pygame.Rect(205, 64, width - 205, height - 194)

    # =========================
    # UI HIT TESTING
    # =========================

    def is_mouse_over_left_panel(self, pos):
        x, y = pos
        return 8 <= x <= 198 and 72 <= y <= 637

    def is_mouse_over_ui(self, pos):
        if getattr(self, "runtime_mode", False):
            return False

        if hasattr(self, "menu_bar") and self.menu_bar.is_mouse_over(pos):
            return True

        if hasattr(self, "toolbar") and self.toolbar.is_mouse_over(pos):
            return True

        if self.is_mouse_over_left_panel(pos):
            return True

        if hasattr(self, "layout_manager"):
            if self.layout_manager.is_mouse_over_any_panel(pos):
                return True

        if self.script_editor.visible:
            return True

        if hasattr(self, "create_asset_modal"):
            if self.create_asset_modal.visible:
                return True

        return False
