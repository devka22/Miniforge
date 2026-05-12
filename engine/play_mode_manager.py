import copy


class PlayModeManager:
    """
    Play Mode seguro.
    FIX:
    - no rompe escenas si grid no tiene serialize/deserialize
    - no conserva cambios hechos en play
    - no duplica control groups
    """

    def __init__(self, game):
        self.game = game
        self.playing = False
        self.paused = False
        self.snapshot = None

    def enter_play_mode(self):
        if self.playing:
            return

        grid_data = None

        if hasattr(self.game.grid, "serialize"):
            grid_data = copy.deepcopy(self.game.grid.serialize())

        self.snapshot = {
            "grid": grid_data,
            "tiles": copy.deepcopy(getattr(self.game.grid, "tiles", None)),
            "entities": [
                copy.deepcopy(unit.serialize())
                for unit in self.game.units
            ],
            "camera": {
                "x": self.game.camera.x,
                "y": self.game.camera.y,
                "zoom": self.game.camera.zoom,
            },
            "control_groups": copy.deepcopy(self.game.serialize_control_groups()),
            "active_tool": self.game.active_tool,
            "tile_brush": self.game.tile_brush,
        }

        self.playing = True
        self.paused = False
        self.game.mode = "PLAY"

        self.game.clear_selection()
        self.game.console.log("PLAY MODE iniciado con snapshot seguro", "ENGINE")

    def exit_play_mode(self):
        if not self.playing:
            return

        from engine.prefab_manager import PrefabManager

        if not self.snapshot:
            self.game.mode = "EDITOR"
            self.playing = False
            return

        grid_data = self.snapshot.get("grid")

        if grid_data and hasattr(self.game.grid, "deserialize"):
            self.game.grid.deserialize(grid_data)

        elif self.snapshot.get("tiles") is not None:
            self.game.grid.tiles = copy.deepcopy(self.snapshot["tiles"])

            if hasattr(self.game.grid, "rebuild_chunks"):
                self.game.grid.rebuild_chunks()

        self.game.units.clear()
        self.game.selected_units.clear()

        for entity_data in self.snapshot["entities"]:
            unit = PrefabManager.entity_from_data(
                self.game,
                entity_data,
                preserve_id=True
            )

            if unit:
                self.game.units.append(unit)

        self.game.world.entities = self.game.units

        camera = self.snapshot["camera"]
        self.game.camera.x = camera["x"]
        self.game.camera.y = camera["y"]
        self.game.camera.zoom = camera["zoom"]

        self.game.deserialize_control_groups(
            self.snapshot.get("control_groups", {})
        )

        self.game.active_tool = self.snapshot.get("active_tool", "Select")
        self.game.tile_brush = self.snapshot.get("tile_brush", 0)

        self.snapshot = None
        self.playing = False
        self.paused = False
        self.game.mode = "EDITOR"

        self.game.console.log("PLAY MODE cerrado. Escena restaurada.", "ENGINE")

    def toggle(self):
        if self.playing:
            self.exit_play_mode()
        else:
            self.enter_play_mode()

    def pause(self):
        if not self.playing:
            return False

        self.paused = True
        self.game.console.log("PLAY MODE pausado", "ENGINE")
        return True

    def resume(self):
        if not self.playing:
            return False

        self.paused = False
        self.game.console.log("PLAY MODE reanudado", "ENGINE")
        return True

    def toggle_pause(self):
        if self.paused:
            return self.resume()
        return self.pause()

    def restart(self):
        if not self.playing:
            self.enter_play_mode()
            return True

        self.exit_play_mode()
        self.enter_play_mode()
        self.game.console.log("PLAY MODE reiniciado", "ENGINE")
        return True
