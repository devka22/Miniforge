class HierarchyManager:
    """
    Parent-child para entidades con transform local/global simple.
    """

    def __init__(self, game):
        self.game = game

    def set_parent(self, child, parent, keep_world=True):
        if not child or child is parent:
            return False

        if parent and self.is_descendant(parent, child):
            self.game.console.log("No se puede crear ciclo en jerarquía", "WARNING")
            return False

        if keep_world:
            child.local_x = getattr(child, "x", 0) - (getattr(parent, "x", 0) if parent else 0)
            child.local_y = getattr(child, "y", 0) - (getattr(parent, "y", 0) if parent else 0)

        child.parent_id = getattr(parent, "id", None) if parent else None
        self.game.mark_scene_dirty("Set Parent")
        return True

    def clear_parent(self, child):
        return self.set_parent(child, None, keep_world=True)

    def children_of(self, parent):
        parent_id = getattr(parent, "id", None)

        return [
            entity for entity in self.game.units
            if getattr(entity, "parent_id", None) == parent_id
        ]

    def parent_of(self, child):
        parent_id = getattr(child, "parent_id", None)

        if not parent_id:
            return None

        return self.game.get_entity_by_id(parent_id)

    def is_descendant(self, entity, possible_parent):
        current = self.parent_of(entity)

        while current:
            if current is possible_parent:
                return True

            current = self.parent_of(current)

        return False

    def sync_child_world_transforms(self):
        for entity in self.game.units:
            parent = self.parent_of(entity)

            if not parent:
                continue

            entity.x = getattr(parent, "x", 0) + getattr(entity, "local_x", 0)
            entity.y = getattr(parent, "y", 0) + getattr(entity, "local_y", 0)

            if hasattr(entity, "sync_to_components"):
                entity.sync_to_components()
