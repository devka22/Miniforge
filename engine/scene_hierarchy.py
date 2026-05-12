class SceneHierarchy:
    """
    Scene Hierarchy Pro 0.5.5 FIX.
    - Búsqueda visual.
    - Filtro por tag.
    - Filtro por layer.
    - Selección con shift.
    - Respeta selection manager.
    - FIX: agrega search_active y search_buffer para evitar pantalla negra.
    """

    def __init__(self, game):
        self.game = game

        self.scroll = 0
        self.max_visible = 7

        self.search_text = ""
        self.filter_tag = "All"
        self.filter_layer = "All"

        # FIX IMPORTANTE
        self.search_active = False
        self.search_buffer = ""

    def ensure_runtime_fields(self):
        """
        Parche de seguridad por si se carga un objeto viejo.
        """
        if not hasattr(self, "search_active"):
            self.search_active = False

        if not hasattr(self, "search_buffer"):
            self.search_buffer = getattr(self, "search_text", "")

        if not hasattr(self, "search_text"):
            self.search_text = ""

        if not hasattr(self, "filter_tag"):
            self.filter_tag = "All"

        if not hasattr(self, "filter_layer"):
            self.filter_layer = "All"

        if not hasattr(self, "scroll"):
            self.scroll = 0

        if not hasattr(self, "max_visible"):
            self.max_visible = 7

    def get_entities(self):
        self.ensure_runtime_fields()

        entities = list(self.game.units)

        if self.search_text:
            query = self.search_text.lower()

            entities = [
                entity for entity in entities
                if query in getattr(entity, "name", "").lower()
                or query in getattr(entity, "id", "").lower()
                or query in getattr(entity, "tag", "").lower()
                or query in getattr(entity, "layer", "").lower()
            ]

        if self.filter_tag != "All":
            entities = [
                entity for entity in entities
                if getattr(entity, "tag", "Untagged") == self.filter_tag
            ]

        if self.filter_layer != "All":
            entities = [
                entity for entity in entities
                if getattr(entity, "layer", "Default") == self.filter_layer
            ]

        return entities

    def depth_of(self, entity):
        depth = 0
        parent_id = getattr(entity, "parent_id", None)
        seen = set()

        while parent_id and parent_id not in seen:
            seen.add(parent_id)
            parent = self.game.get_entity_by_id(parent_id)

            if not parent:
                break

            depth += 1
            parent_id = getattr(parent, "parent_id", None)

        return min(depth, 6)

    def scroll_up(self):
        self.ensure_runtime_fields()
        self.scroll = max(0, self.scroll - 1)

    def scroll_down(self):
        self.ensure_runtime_fields()

        entities = self.get_entities()
        max_scroll = max(0, len(entities) - self.max_visible)
        self.scroll = min(max_scroll, self.scroll + 1)

    def select_by_index(self, visible_index, shift=False):
        self.ensure_runtime_fields()

        entities = self.get_entities()
        index = self.scroll + visible_index

        if index < 0 or index >= len(entities):
            return

        entity = entities[index]

        if hasattr(self.game, "selection_manager"):
            if shift:
                self.game.selection_manager.toggle(entity)
            else:
                self.game.selection_manager.clear()
                self.game.selection_manager.add(entity)
        else:
            if not shift:
                self.game.clear_selection()

            if shift and entity in self.game.selected_units:
                self.game.remove_from_selection(entity)
            else:
                self.game.add_to_selection(entity)

    def set_search(self, text):
        self.ensure_runtime_fields()

        self.search_text = str(text)
        self.search_buffer = str(text)
        self.scroll = 0

    def clear_search(self):
        self.search_text = ""
        self.search_buffer = ""
        self.search_active = False
        self.scroll = 0

    def begin_search(self):
        self.ensure_runtime_fields()

        self.search_active = True
        self.search_buffer = self.search_text

    def end_search(self, apply=True):
        self.ensure_runtime_fields()

        if apply:
            self.search_text = self.search_buffer

        self.search_active = False
        self.scroll = 0

    def handle_search_key(self, event):
        """
        Retorna True si consumió el evento.
        """
        self.ensure_runtime_fields()

        if not self.search_active:
            return False

        import pygame

        if event.key == pygame.K_ESCAPE:
            self.clear_search()
            return True

        if event.key == pygame.K_RETURN:
            self.end_search(True)
            return True

        if event.key == pygame.K_BACKSPACE:
            self.search_buffer = self.search_buffer[:-1]
            self.search_text = self.search_buffer
            self.scroll = 0
            return True

        if event.unicode and event.unicode.isprintable():
            self.search_buffer += event.unicode
            self.search_text = self.search_buffer
            self.scroll = 0
            return True

        return True

    def cycle_tag_filter(self):
        self.ensure_runtime_fields()

        tags = ["All"] + list(getattr(self.game, "tags", []))

        if self.filter_tag not in tags:
            self.filter_tag = "All"
            return

        index = tags.index(self.filter_tag)
        self.filter_tag = tags[(index + 1) % len(tags)]
        self.scroll = 0

        if hasattr(self.game, "console"):
            self.game.console.log(
                f"Hierarchy Tag Filter: {self.filter_tag}",
                "EDITOR"
            )

    def cycle_layer_filter(self):
        self.ensure_runtime_fields()

        layers = ["All"] + list(getattr(self.game, "layers", []))

        if self.filter_layer not in layers:
            self.filter_layer = "All"
            return

        index = layers.index(self.filter_layer)
        self.filter_layer = layers[(index + 1) % len(layers)]
        self.scroll = 0

        if hasattr(self.game, "console"):
            self.game.console.log(
                f"Hierarchy Layer Filter: {self.filter_layer}",
                "EDITOR"
            )

    def reset_filters(self):
        self.search_text = ""
        self.search_buffer = ""
        self.search_active = False
        self.filter_tag = "All"
        self.filter_layer = "All"
        self.scroll = 0

        if hasattr(self.game, "console"):
            self.game.console.log(
                "Hierarchy filters reset",
                "EDITOR"
            )
