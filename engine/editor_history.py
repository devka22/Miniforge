import copy
from entities.unit import Unit


class EditorHistory:
    """
    Sistema de Undo/Redo del editor.
    Guarda snapshots simples de la escena.
    """

    def __init__(self, game, max_steps=30):
        self.game = game
        self.max_steps = max_steps

        self.undo_stack = []
        self.redo_stack = []

    def take_snapshot(self, label="Change"):
        snapshot = {
            "label": label,
            "units": []
        }

        for unit in self.game.units:
            snapshot["units"].append(unit.serialize())

        self.undo_stack.append(copy.deepcopy(snapshot))
        self.redo_stack.clear()

        if len(self.undo_stack) > self.max_steps:
            self.undo_stack.pop(0)

        self.game.console.log(f"Snapshot: {label}")

    def undo(self):
        if len(self.undo_stack) <= 1:
            self.game.console.log("No hay más undo")
            return

        current = self.undo_stack.pop()
        self.redo_stack.append(current)

        previous = self.undo_stack[-1]
        self.restore_snapshot(previous)

        self.game.console.log("Undo aplicado")

    def redo(self):
        if not self.redo_stack:
            self.game.console.log("No hay redo")
            return

        snapshot = self.redo_stack.pop()
        self.undo_stack.append(snapshot)

        self.restore_snapshot(snapshot)

        self.game.console.log("Redo aplicado")

    def restore_snapshot(self, snapshot):
        self.game.units.clear()
        self.game.selected_units.clear()

        for data in snapshot.get("units", []):
            unit = Unit(
                data.get("x", 0),
                data.get("y", 0),
                self.game
            )

            unit.speed = data.get("speed", 3.5)

            for script_data in data.get("scripts", []):
                script_name = script_data.get("script")
                script = self.game.script_manager.create(script_name)

                if script:
                    script.deserialize(script_data)
                    unit.add_script(script)

            self.game.units.append(unit)

        self.game.world.entities = self.game.units