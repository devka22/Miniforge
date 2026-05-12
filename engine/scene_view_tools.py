import copy

from engine.prefab_manager import PrefabManager


class SceneViewTools:
    """
    Herramientas avanzadas de Scene View.
    Move/Rotate/Scale metadata, snapping, align/distribute y hierarchy ops.
    """

    def __init__(self, game):
        self.game = game
        self.gizmo_mode = "Move"
        self.grid_snapping = True
        self.snap_size = 1.0
        self.drag_duplicate = False

    def cycle_gizmo_mode(self):
        modes = ["Move", "Rotate", "Scale"]
        index = modes.index(self.gizmo_mode) if self.gizmo_mode in modes else 0
        self.gizmo_mode = modes[(index + 1) % len(modes)]
        self.game.active_tool = self.gizmo_mode
        self.game.console.log(f"Gizmo: {self.gizmo_mode}", "EDITOR")
        return self.gizmo_mode

    def toggle_snapping(self):
        self.grid_snapping = not self.grid_snapping
        self.game.console.log(f"Grid snapping: {self.grid_snapping}", "EDITOR")
        return self.grid_snapping

    def set_snap_size(self, snap_size):
        self.snap_size = max(0.01, float(snap_size))
        self.game.console.log(f"Snap size: {self.snap_size}", "EDITOR")
        return self.snap_size

    def snap_value(self, value):
        if not self.grid_snapping:
            return value

        return round(value / self.snap_size) * self.snap_size

    def snap_selected(self):
        for entity in self.game.selected_units:
            entity.x = self.snap_value(entity.x)
            entity.y = self.snap_value(entity.y)

            if hasattr(entity, "sync_to_components"):
                entity.sync_to_components()

        self.game.mark_scene_dirty("Snap Selected")
        self.game.history.take_snapshot("Snap Selected")

    def apply_screen_drag(self, dx, dy, mode=None):
        if not self.game.selected_units:
            return False

        mode = mode or self.gizmo_mode
        zoom = max(0.001, getattr(self.game.camera, "zoom", 1.0))
        tile = max(1, getattr(self.game.grid, "tile_size", 32))
        world_dx = dx / zoom / tile
        world_dy = dy / zoom / tile

        for entity in self.game.selected_units:
            if mode == "Move":
                entity.x += world_dx
                entity.y += world_dy

                if self.grid_snapping:
                    entity.x = self.snap_value(entity.x)
                    entity.y = self.snap_value(entity.y)

            elif mode == "Rotate":
                entity.rotation = getattr(entity, "rotation", 0.0) + dx * 0.35

            elif mode == "Scale":
                amount = 1.0 + (dx - dy) * 0.005
                entity.scale_x = max(0.05, getattr(entity, "scale_x", 1.0) * amount)
                entity.scale_y = max(0.05, getattr(entity, "scale_y", 1.0) * amount)

            if hasattr(entity, "sync_to_components"):
                entity.sync_to_components()

        self.game.mark_scene_dirty(f"Gizmo {mode}")
        return True

    def align_selected(self, axis="x"):
        if len(self.game.selected_units) < 2:
            self.game.console.log("Selecciona al menos dos entidades", "WARNING")
            return

        target = getattr(self.game.selected_units[0], axis, 0)

        for entity in self.game.selected_units[1:]:
            setattr(entity, axis, target)

            if hasattr(entity, "sync_to_components"):
                entity.sync_to_components()

        self.game.mark_scene_dirty(f"Align {axis.upper()}")
        self.game.history.take_snapshot(f"Align {axis.upper()}")

    def distribute_selected(self, axis="x"):
        entities = sorted(
            self.game.selected_units,
            key=lambda entity: getattr(entity, axis, 0)
        )

        if len(entities) < 3:
            self.game.console.log("Selecciona al menos tres entidades", "WARNING")
            return

        start = getattr(entities[0], axis, 0)
        end = getattr(entities[-1], axis, 0)
        step = (end - start) / (len(entities) - 1)

        for index, entity in enumerate(entities):
            setattr(entity, axis, start + step * index)

            if hasattr(entity, "sync_to_components"):
                entity.sync_to_components()

        self.game.mark_scene_dirty(f"Distribute {axis.upper()}")
        self.game.history.take_snapshot(f"Distribute {axis.upper()}")

    def duplicate_selected_with_children(self):
        originals = list(self.game.selected_units)

        if not originals:
            self.game.console.log("No hay selección", "WARNING")
            return []

        all_entities = []

        for entity in originals:
            all_entities.extend(self.collect_with_children(entity))

        all_entities = list(dict.fromkeys(all_entities))
        id_map = {}
        duplicates = []

        for entity in all_entities:
            duplicate = PrefabManager.entity_from_data(
                self.game,
                entity.serialize(),
                preserve_id=False
            )

            if not duplicate:
                continue

            duplicate.x += 1
            duplicate.y += 1
            duplicate.local_x = getattr(entity, "local_x", 0)
            duplicate.local_y = getattr(entity, "local_y", 0)
            id_map[getattr(entity, "id", None)] = duplicate.id
            duplicates.append(duplicate)

        for duplicate in duplicates:
            if getattr(duplicate, "parent_id", None) in id_map:
                duplicate.parent_id = id_map[duplicate.parent_id]

            self.game.units.append(duplicate)

        self.game.world.entities = self.game.units
        self.game.clear_selection()

        for duplicate in duplicates:
            self.game.add_to_selection(duplicate)

        self.game.mark_scene_dirty("Duplicate Hierarchy")
        self.game.history.take_snapshot("Duplicate Hierarchy")
        return duplicates

    def delete_selected_with_children(self):
        targets = []

        for entity in self.game.selected_units:
            targets.extend(self.collect_with_children(entity))

        targets = list(dict.fromkeys(targets))

        for entity in targets:
            if entity in self.game.units:
                self.game.units.remove(entity)

        self.game.selected_units.clear()
        self.game.world.entities = self.game.units
        self.game.mark_scene_dirty("Delete Hierarchy")
        self.game.history.take_snapshot("Delete Hierarchy")

    def collect_with_children(self, entity):
        result = [entity]

        for child in self.game.hierarchy_manager.children_of(entity):
            result.extend(self.collect_with_children(child))

        return result

    def create_empty_child(self):
        if not self.game.selected_units:
            self.game.console.log("Selecciona un parent primero", "WARNING")
            return None

        parent = self.game.selected_units[-1]
        child = self.game.api.create_game_object("Child", parent.x, parent.y)
        self.game.hierarchy_manager.set_parent(child, parent, keep_world=True)
        self.game.clear_selection()
        self.game.add_to_selection(child)
        self.game.history.take_snapshot("Create Empty Child")
        return child

    def set_hierarchy_enabled(self, entity, enabled):
        for item in self.collect_with_children(entity):
            item.enabled = enabled

        self.game.mark_scene_dirty("Hierarchy Enabled")

    def set_hierarchy_visible(self, entity, visible):
        for item in self.collect_with_children(entity):
            item.visible = visible

        self.game.mark_scene_dirty("Hierarchy Visible")

    def set_hierarchy_locked(self, entity, locked):
        for item in self.collect_with_children(entity):
            item.locked = locked

        self.game.mark_scene_dirty("Hierarchy Locked")
