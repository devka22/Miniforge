import os
import tempfile
import unittest

from entities.game_object import GameObject, game_object_from_data
from engine.input_map import InputMap
from engine.build_profiles import BuildProfiles
from engine.hierarchy_manager import HierarchyManager
from engine.prefab_overrides import PrefabOverrides
from engine.system_scheduler import SystemScheduler
from engine.script_editor import ScriptEditor
from engine.visual_input_editor import VisualInputEditor
from engine.component import Rigidbody2D, component_from_data
from engine.tilemap_layers import TilemapLayers
from engine.audio_mixer import AudioMixer
from engine.component_tools import ComponentTools
from engine.build_report import BuildReport
from systems.physics_system import PhysicsSystem
from engine.animation_graph import AnimationGraphLibrary
from engine.ui_canvas import UICanvas
from engine.visual_scripting import VisualScriptRuntime
from engine.advanced_prefabs import AdvancedPrefabSystem
from engine.profiler import Profiler
from systems.animation_system import AnimationSystem
from engine.component_validation import ComponentValidation
from engine.scene_serializer import SceneSerializer
from engine.resource_manager import ResourceManager
from engine.game_api import GameAPI
from engine.upgrade_manifest import EngineUpgradeManifest
from systems.gameplay_system import GameplaySystem
from engine.asset_tools import AssetTools
from engine.file_browser import FileBrowser
from engine.play_mode_manager import PlayModeManager
from engine.version import ENGINE_VERSION
from engine.asset_database import AssetDatabase
from engine.plugin_manager import PluginManager


class Engine060Tests(unittest.TestCase):
    def test_game_object_roundtrip(self):
        obj = GameObject(3, 4, name="Crate")
        data = obj.serialize()

        class DummyGame:
            script_manager = None

        game = DummyGame()
        game.script_manager = type("Scripts", (), {"create": lambda self, name: None})()
        clone = game_object_from_data(game, data, preserve_id=True)

        self.assertEqual(clone.name, "Crate")
        self.assertEqual(clone.x, 3)
        self.assertEqual(clone.get_component("Transform").x, 3)

    def test_input_map_persists_bindings(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "input_map.json")
            input_map = InputMap(path)
            input_map.set_binding("dash", ["left_shift"])

            loaded = InputMap(path)
            self.assertEqual(loaded.bindings["dash"], ["left_shift"])

    def test_build_profiles_cycle(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "build_profiles.json")
            profiles = BuildProfiles(path)
            first = profiles.active
            second = profiles.cycle()

            self.assertNotEqual(first, second)

    def test_hierarchy_parenting(self):
        parent = GameObject(10, 10, name="Parent")
        child = GameObject(12, 13, name="Child")

        class DummyGame:
            def __init__(self):
                self.units = [parent, child]
                self.console = type("Console", (), {"log": lambda *args: None})()

            def get_entity_by_id(self, entity_id):
                for entity in self.units:
                    if entity.id == entity_id:
                        return entity

            def mark_scene_dirty(self, reason="Change"):
                self.dirty = True

        game = DummyGame()
        hierarchy = HierarchyManager(game)
        hierarchy.set_parent(child, parent)
        hierarchy.sync_child_world_transforms()

        self.assertEqual(child.parent_id, parent.id)
        self.assertEqual(child.x, 12)
        self.assertEqual(child.y, 13)

    def test_prefab_override_diff(self):
        obj = GameObject(1, 2, name="Crate")

        class DummyGame:
            pass

        overrides = PrefabOverrides(DummyGame())
        diff = overrides.diff_dict("", {"name": "A"}, {"name": "B"})
        self.assertEqual(diff[0]["path"], "name")

    def test_system_scheduler_priority(self):
        calls = []

        class DummyGame:
            error_handler = type(
                "Errors",
                (),
                {"safe_call": lambda self, name, callback, dt: callback(dt)},
            )()

        class System:
            def __init__(self, name):
                self.name = name

            def update(self, dt):
                calls.append(self.name)

        scheduler = SystemScheduler(DummyGame())
        scheduler.register(System("late"), priority=20)
        scheduler.register(System("early"), priority=1)
        scheduler.update(0.016)

        self.assertEqual(calls, ["early", "late"])

    def test_script_editor_validates_syntax(self):
        class DummyGame:
            project_path = "."
            console = type("Console", (), {"log": lambda *args: None})()

            def __init__(self):
                self.script_manager = type("Scripts", (), {"scan_scripts": lambda *args, **kwargs: None})()
                self.asset_database = type("Assets", (), {"scan": lambda *args: None})()
                self.file_browser = type("Browser", (), {"refresh": lambda *args: None})()

        editor = ScriptEditor(DummyGame())
        editor.lines = ["def broken("]
        self.assertFalse(editor.validate())
        self.assertIsNotNone(editor.document.syntax_error)

    def test_visual_input_editor_adds_binding(self):
        class DummyInput:
            def __init__(self):
                self.bindings = {"jump": ["space"]}

            def set_binding(self, action, keys):
                self.bindings[action] = keys

        class DummyGame:
            input_map = DummyInput()
            console = type("Console", (), {"log": lambda *args: None})()

        editor = VisualInputEditor(DummyGame())
        editor.select("jump")
        editor.add_binding("j")
        self.assertIn("j", DummyGame.input_map.bindings["jump"])

    def test_rigidbody_roundtrip_and_force(self):
        body = Rigidbody2D()
        body.add_force(4, 2, impulse=True)
        clone = component_from_data(body.serialize())

        self.assertEqual(clone.component_type, "Rigidbody2D")
        self.assertEqual(clone.velocity_x, 4)
        self.assertEqual(clone.velocity_y, 2)

    def test_tilemap_layers_paint_and_serialize(self):
        tilemap = TilemapLayers(8, 8)
        tilemap.set_tile(2, 3, 4)
        tilemap.cycle_layer()
        tilemap.fill_active(0, 0, 2, 2, 1)
        clone = TilemapLayers(1, 1)
        clone.deserialize(tilemap.serialize())

        self.assertEqual(clone.layer("Ground").get(2, 3), 4)
        self.assertEqual(clone.stats()["layers"], 4)

    def test_component_tools_copy_paste(self):
        source = GameObject(0, 0, name="Source")
        target = GameObject(1, 1, name="Target")

        class DummyGame:
            def __init__(self):
                self.selected_units = [source]
                self.component_registry = type(
                    "Registry",
                    (),
                    {"create": lambda self, name: component_from_data({"component_type": name})},
                )()
                self.console = type("Console", (), {"log": lambda *args: None})()

            def mark_scene_dirty(self, reason="Change"):
                self.dirty = True

        game = DummyGame()
        tools = ComponentTools(game)
        source.add_component(Rigidbody2D())
        tools.copy("Rigidbody2D")
        game.selected_units = [target]
        tools.paste()

        self.assertIsNotNone(target.get_component("Rigidbody2D"))

    def test_audio_mixer_serializes_buses(self):
        mixer = AudioMixer()
        mixer.set_bus_volume("SFX", 0.25)
        data = mixer.serialize()
        clone = AudioMixer()
        clone.deserialize(data)

        self.assertEqual(clone.buses["SFX"].volume, 0.25)

    def test_physics_integrates_rigidbody(self):
        entity = GameObject(0, 0, name="Body")
        body = Rigidbody2D()
        body.use_gravity = False
        body.velocity_x = 10
        entity.add_component(body)

        class DummyGame:
            mode = "PLAY"
            grid = type("Grid", (), {"tile_size": 32})()
            console = type("Console", (), {"log": lambda *args: None})()

            def __init__(self):
                self.world = type("World", (), {"entities": [entity]})()

        physics = PhysicsSystem(DummyGame())
        physics.update(0.1)

        self.assertGreater(entity.x, 0)

    def test_build_report_summary(self):
        with tempfile.TemporaryDirectory() as tmp:
            with open(os.path.join(tmp, "run_game.py"), "w", encoding="utf-8") as file:
                file.write("# run")

            class DummySettings:
                def get(self, key, default=None):
                    return default

            class DummyGame:
                project_path = tmp
                units = []
                build_settings = DummySettings()
                asset_database = type("Assets", (), {"assets": []})()
                build_profiles = type("Profiles", (), {"active": "Development"})()
                console = type("Console", (), {"log": lambda *args: None})()

                def project_join(self, *parts):
                    return os.path.join(tmp, *parts)

            report = BuildReport(DummyGame()).generate(tmp)

            self.assertGreaterEqual(report["summary"]["files"], 1)
            self.assertTrue(os.path.exists(os.path.join(tmp, "build_report.json")))

    def test_animation_system_applies_tint(self):
        entity = GameObject(0, 0, name="Animated")
        entity.add_component(component_from_data({"component_type": "Animator"}))

        class DummyGame:
            mode = "PLAY"
            animation_graphs = AnimationGraphLibrary()

            def __init__(self):
                self.world = type("World", (), {"entities": [entity]})()

        AnimationSystem(DummyGame()).update(0.6)

        self.assertNotEqual(entity.get_component("SpriteRenderer").tint, (255, 255, 255))

    def test_ui_canvas_hit_test(self):
        entity = GameObject(0, 0, name="Button")
        entity.add_component(component_from_data({
            "component_type": "UIElement",
            "element_type": "Button",
            "x": 10,
            "y": 10,
            "width": 100,
            "height": 40,
        }))

        class DummyScreen:
            def get_rect(self):
                return type("Rect", (), {"width": 400, "height": 300, "centerx": 200, "centery": 150})()

        class DummyGame:
            screen = DummyScreen()

            def __init__(self):
                self.world = type("World", (), {"entities": [entity]})()

        canvas = UICanvas(DummyGame())
        hit_entity, _ = canvas.hit_test((20, 20))

        self.assertEqual(hit_entity, entity)

    def test_visual_scripting_moves_entity(self):
        entity = GameObject(0, 0, name="Scripted")
        script = component_from_data({"component_type": "VisualScript"})
        script.nodes = [
            {"id": "start", "type": "EventStart", "next": "move"},
            {"id": "move", "type": "Move", "x": 3, "y": 4, "next": None},
        ]
        entity.add_component(script)

        class DummyGame:
            mode = "PLAY"
            console = type("Console", (), {"log": lambda *args: None})()

            def __init__(self):
                self.world = type("World", (), {"entities": [entity]})()

        VisualScriptRuntime(DummyGame()).update(0.016)

        self.assertEqual(entity.x, 3)
        self.assertEqual(entity.y, 4)

    def test_advanced_prefab_variant_diff(self):
        with tempfile.TemporaryDirectory() as tmp:
            old_cwd = os.getcwd()
            os.chdir(tmp)

            try:
                entity = GameObject(0, 0, name="VariantSource")

                class DummyGame:
                    def __init__(self):
                        self.selected_units = [entity]
                        self.console = type("Console", (), {"log": lambda *args: None})()
                        self.asset_database = type("Assets", (), {"scan": lambda *args: None})()

                    def refresh_project(self):
                        self.refreshed = True

                system = AdvancedPrefabSystem(DummyGame())
                path = system.create_variant_from_selected()

                self.assertTrue(os.path.exists(path))
                self.assertEqual(system.stats["variants_created"], 1)
            finally:
                os.chdir(old_cwd)

    def test_profiler_records_scheduler_time(self):
        profiler = Profiler()
        profiler.record_system("SystemA", 1.5)
        profiler.begin_frame()
        profiler.end_frame()

        self.assertIn(("SystemA", "1.5 ms"), profiler.rows())

    def test_scheduler_respects_editor_play_modes(self):
        calls = []

        class DummyGame:
            mode = "EDITOR"
            profiler = None
            error_handler = type(
                "Errors",
                (),
                {
                    "last_call_failed": False,
                    "safe_call": lambda self, name, callback, dt: callback(dt),
                },
            )()

        class PlayOnly:
            run_in_editor = False
            run_in_play = True

            def update(self, dt):
                calls.append("play")

        scheduler = SystemScheduler(DummyGame())
        scheduler.register(PlayOnly())
        scheduler.update(0.016)

        self.assertEqual(calls, [])

    def test_component_validation_repairs_ranges(self):
        audio = component_from_data({
            "component_type": "AudioSource",
            "volume": 5,
            "spatial_blend": -1,
        })
        changed = ComponentValidation.repair_component(audio)

        self.assertTrue(changed)
        self.assertEqual(audio.volume, 1.0)
        self.assertEqual(audio.spatial_blend, 0.0)

    def test_scene_serializer_migrates_old_data(self):
        data = SceneSerializer.migrate({"objects": [{"name": "Old"}]})

        self.assertEqual(data["version"], ENGINE_VERSION)
        self.assertEqual(data["entities"][0]["name"], "Old")
        self.assertEqual(data["brush_size"], 1)

    def test_resource_manager_recursive_scan(self):
        with tempfile.TemporaryDirectory() as tmp:
            sprites = os.path.join(tmp, "sprites", "nested")
            os.makedirs(sprites)
            path = os.path.join(sprites, "hero.png")
            with open(path, "wb") as file:
                file.write(b"not-a-real-image")

            manager = ResourceManager(tmp)
            manager.load_image = lambda name, rel_path: manager.images.setdefault(name, rel_path)
            manager.scan_sprites()

            self.assertEqual(manager.images["hero"], os.path.join("sprites", "nested", "hero.png"))

    def test_upgrade_manifest_tracks_more_than_100_improvements(self):
        manifest = EngineUpgradeManifest()
        summary = manifest.summary()

        self.assertGreaterEqual(summary["count"], 100)
        self.assertGreaterEqual(summary["advanced_components"], 30)

    def test_advanced_components_roundtrip_inventory_and_ai(self):
        inventory = component_from_data({
            "component_type": "Inventory",
            "capacity": 2,
            "items": [],
        })
        added = inventory.add_item("potion", 3)
        clone = component_from_data(inventory.serialize())

        ai = component_from_data({
            "component_type": "AIController",
            "behavior": "attack",
            "target_tags": ["Player"],
        })

        self.assertEqual(added, 3)
        self.assertTrue(clone.has_item("potion", 3))
        self.assertEqual(ai.behavior, "attack")
        self.assertEqual(ai.target_tags, ["Player"])

    def test_gameplay_system_lifetime_destroys_entity(self):
        entity = GameObject(0, 0, name="Temp")
        entity.add_component(component_from_data({
            "component_type": "Lifetime",
            "duration": 0.01,
        }))

        class DummyGame:
            mode = "PLAY"
            selected_units = []

            def __init__(self):
                self.units = [entity]
                self.world = type("World", (), {"entities": self.units})()

        game = DummyGame()
        GameplaySystem(game).update(0.02)

        self.assertEqual(game.units, [])

    def test_gameplay_ai_damages_target(self):
        attacker = GameObject(0, 0, name="Attacker")
        attacker.add_component(component_from_data({
            "component_type": "AIController",
            "behavior": "attack",
            "target_tags": ["Enemy"],
            "detection_radius": 5,
            "attack_radius": 2,
            "think_interval": 0.02,
        }))
        attacker.add_component(component_from_data({
            "component_type": "DamageDealer",
            "damage": 12,
            "cooldown": 0,
            "target_tags": ["Enemy"],
        }))

        target = GameObject(1, 0, name="Target")
        target.tag = "Enemy"
        target.add_component(component_from_data({
            "component_type": "Health",
            "max_health": 50,
        }))

        class DummyGame:
            mode = "PLAY"
            selected_units = []
            event_bus = type("Events", (), {"emit": lambda *args, **kwargs: None})()

            def __init__(self):
                self.units = [attacker, target]
                self.world = type("World", (), {"entities": self.units})()

            def get_entity_by_id(self, entity_id):
                for entity in self.units:
                    if entity.id == entity_id:
                        return entity

        GameplaySystem(DummyGame()).update(0.05)

        self.assertLess(target.get_component("Health").health, 50)

    def test_game_api_inventory_cooldown_and_tween_helpers(self):
        entity = GameObject(0, 0, name="Player")

        class DummyGame:
            def __init__(self):
                self.units = [entity]
                self.world = type("World", (), {"entities": self.units})()

            def mark_scene_dirty(self, reason="Change"):
                self.dirty = reason

        api = GameAPI(DummyGame())
        api.add_item(entity, "key", 2)
        api.start_cooldown(entity, "dash", 1.0)
        api.tween(entity, "x", 10, duration=0.5)

        self.assertEqual(api.item_count(entity, "key"), 2)
        self.assertFalse(api.cooldown_ready(entity, "dash"))
        self.assertTrue(entity.get_component("Tween").active)

    def test_beta_entity_serializes_standard_fields(self):
        entity = GameObject(5, 7, name="Player")
        entity.rotation = 30
        entity.scale_x = 2
        entity.scale_y = 3
        entity.width = 32
        entity.height = 48
        entity.script = "player.py"
        entity.sync_to_components()
        data = entity.serialize()

        self.assertEqual(data["position"], [5, 7])
        self.assertEqual(data["scale"], [2, 3])
        self.assertEqual(data["size"], [32, 48])
        self.assertEqual(data["script"], "player.py")
        self.assertTrue(data["active"])

    def test_beta_scene_template_has_required_json_shape(self):
        data = AssetTools.template_scene("main_scene")

        self.assertEqual(data["scene_name"], "main_scene")
        self.assertEqual(data["engine_version"], ENGINE_VERSION)
        self.assertIn("entities", data)
        self.assertIn("tiles", data)
        self.assertIn("camera", data)
        self.assertIn("settings", data)

    def test_beta_file_browser_delete_requires_confirmation(self):
        with tempfile.TemporaryDirectory() as tmp:
            AssetTools.ensure_project_folders(tmp)
            script_path = AssetTools.create_script(tmp, "DeleteMe")

            class DummyGame:
                project_path = tmp
                console = type("Console", (), {"log": lambda *args: None})()

            browser = FileBrowser(DummyGame())
            browser.select_asset_by_path(script_path)

            self.assertFalse(browser.delete_selected_asset())
            self.assertTrue(os.path.exists(script_path))
            self.assertTrue(browser.delete_selected_asset())
            self.assertFalse(os.path.exists(script_path))

    def test_beta_project_files_include_engine_config(self):
        with tempfile.TemporaryDirectory() as tmp:
            AssetTools.ensure_project_folders(tmp)

            self.assertTrue(os.path.exists(os.path.join(tmp, "project.json")))
            self.assertTrue(os.path.exists(os.path.join(tmp, "engine_config.json")))
            self.assertTrue(os.path.exists(os.path.join(tmp, "logs")))
            self.assertTrue(os.path.exists(os.path.join(tmp, "saves", "scenes")))

    def test_beta_play_mode_restores_snapshot(self):
        entity = GameObject(1, 1, name="Player")

        class DummyCamera:
            x = 0
            y = 0
            zoom = 1

        class DummyGrid:
            tiles = [[0]]

        class DummyGame:
            mode = "EDITOR"
            camera = DummyCamera()
            grid = DummyGrid()
            active_tool = "Select"
            tile_brush = 0
            selected_units = []
            console = type("Console", (), {"log": lambda *args: None})()

            def __init__(self):
                self.units = [entity]
                self.world = type("World", (), {"entities": self.units})()

            def serialize_control_groups(self):
                return {}

            def deserialize_control_groups(self, data):
                self.groups = data

            def clear_selection(self):
                self.selected_units = []

        game = DummyGame()
        manager = PlayModeManager(game)
        manager.enter_play_mode()
        game.units[0].x = 99
        manager.exit_play_mode()

        self.assertEqual(game.mode, "EDITOR")
        self.assertEqual(game.units[0].x, 1)

    def test_asset_database_import_settings_and_dependencies(self):
        with tempfile.TemporaryDirectory() as tmp:
            assets = os.path.join(tmp, "assets", "data")
            scenes = os.path.join(tmp, "saves", "scenes")
            os.makedirs(assets)
            os.makedirs(scenes)

            item_path = os.path.join(assets, "Items.json")
            scene_path = os.path.join(scenes, "main.scene")

            with open(item_path, "w", encoding="utf-8") as file:
                file.write("{}")

            with open(scene_path, "w", encoding="utf-8") as file:
                file.write('{"uses": "Items"}')

            database = AssetDatabase(os.path.join(tmp, "assets"), tmp)
            database.set_import_setting(os.path.join("assets", "data", "Items.json"), "include_in_build", False)
            graph = database.rebuild_dependency_graph()

            self.assertFalse(database.get_import_settings(os.path.join("assets", "data", "Items.json"))["include_in_build"])
            self.assertIn(os.path.join("assets", "data", "Items.json"), graph[os.path.join("saves", "scenes", "main.scene")])

    def test_plugin_manager_emits_hooks(self):
        with tempfile.TemporaryDirectory() as tmp:
            plugin_dir = os.path.join(tmp, "plugins", "hello")
            os.makedirs(plugin_dir)

            with open(os.path.join(plugin_dir, "plugin.json"), "w", encoding="utf-8") as file:
                file.write('{"name": "hello", "enabled": true}')

            with open(os.path.join(plugin_dir, "plugin.py"), "w", encoding="utf-8") as file:
                file.write("def on_editor_start(game):\n    game.called = True\n")

            class DummyGame:
                console = type("Console", (), {"log": lambda *args: None})()

            game = DummyGame()
            manager = PluginManager(tmp)
            count = manager.emit_hook("on_editor_start", game)

            self.assertEqual(count, 1)
            self.assertTrue(game.called)

    def test_scene_view_tools_drag_moves_selected(self):
        entity = GameObject(1, 1, name="Mover")

        class DummyGame:
            def __init__(self):
                self.selected_units = [entity]
                self.camera = type("Camera", (), {"zoom": 1})()
                self.grid = type("Grid", (), {"tile_size": 32})()
                self.console = type("Console", (), {"log": lambda *args: None})()
                self.history = type("History", (), {"take_snapshot": lambda *args: None})()

            def mark_scene_dirty(self, reason="Change"):
                self.dirty = reason

        from engine.scene_view_tools import SceneViewTools

        tools = SceneViewTools(DummyGame())
        tools.grid_snapping = False
        tools.apply_screen_drag(32, 0, "Move")

        self.assertEqual(round(entity.x, 3), 2.0)

    def test_game_ui_and_visual_template_helpers(self):
        class DummyGame:
            def __init__(self):
                self.units = []
                self.world = type("World", (), {"entities": self.units})()
                self.console = type("Console", (), {"log": lambda *args: None})()
                self.component_registry = type(
                    "Registry",
                    (),
                    {"create": lambda self, name: component_from_data({"component_type": name})},
                )()
                self.history = type("History", (), {"take_snapshot": lambda *args: None})()
                self.selected_units = []

            def mark_scene_dirty(self, reason="Change"):
                self.dirty = reason

        game = DummyGame()
        game.api = GameAPI(game)

        from core.game import Game

        label = Game.create_ui_label(game, "Score", 10, 10)
        game.selected_units = [label]
        visual = Game.add_visual_script_template(game, "Button Click")

        self.assertEqual(label.get_component("UIElement").text, "Score")
        self.assertEqual(visual.graph_name, "ButtonClick")


if __name__ == "__main__":
    unittest.main()
