import os
import json

from engine.prefab_manager import PrefabManager


class PrefabWorkflow:
    """
    Prefab Workflow 3 estable.
    """

    def __init__(self, game):
        self.game = game

    def apply_selected_to_prefab(self):
        if not self.game.selected_units:
            self.game.console.log("Selecciona una entidad primero", "WARNING")
            return

        unit = self.game.selected_units[0]
        prefab_source = getattr(unit, "prefab_source", None)

        if not prefab_source:
            self.game.console.log("La entidad no viene de un prefab", "WARNING")
            return

        if not os.path.exists(prefab_source):
            self.game.console.log("No existe el prefab original", "ERROR")
            return

        data = {
            "version": "0.6.0",
            "prefab_name": os.path.basename(prefab_source),
            "entity": unit.serialize()
        }

        try:
            with open(prefab_source, "w", encoding="utf-8") as file:
                json.dump(data, file, indent=4)

            self.game.refresh_project()
            self.game.console.log("Cambios aplicados al prefab", "ASSET")

        except Exception as error:
            self.game.console.log(f"No se pudo aplicar prefab: {error}", "ERROR")

    def revert_selected_prefab(self):
        if not self.game.selected_units:
            self.game.console.log("Selecciona una entidad primero", "WARNING")
            return

        unit = self.game.selected_units[0]
        prefab_source = getattr(unit, "prefab_source", None)

        if not prefab_source:
            self.game.console.log("La entidad no viene de un prefab", "WARNING")
            return

        if not os.path.exists(prefab_source):
            self.game.console.log("No existe el prefab original", "ERROR")
            return

        new_unit = PrefabManager.load_prefab(self.game, prefab_source)

        if not new_unit:
            return

        old_x = unit.x
        old_y = unit.y
        old_id = unit.id
        old_name = unit.name
        old_selected = unit.selected

        new_unit.x = old_x
        new_unit.y = old_y
        new_unit.id = old_id
        new_unit.name = old_name
        new_unit.selected = old_selected
        new_unit.prefab_source = prefab_source
        new_unit.is_prefab_instance = True

        new_unit.sync_to_components()

        try:
            index = self.game.units.index(unit)
            self.game.units[index] = new_unit
        except ValueError:
            self.game.units.append(new_unit)

        self.game.world.entities = self.game.units

        self.game.clear_selection()
        self.game.add_to_selection(new_unit)

        self.game.history.take_snapshot("Revert Prefab")
        self.game.console.log("Prefab revertido", "ASSET")