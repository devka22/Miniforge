import pygame


class UICanvas:
    def __init__(self, game):
        self.game = game
        self.reference_width = 1100
        self.reference_height = 740
        self.scale_mode = "screen"
        self.show_bounds = True
        self.hover_entity_id = None
        self.focus_entity_id = None
        self.stats = {"elements": 0, "interactive": 0}

    def element_rect(self, element):
        screen = self.game.screen.get_rect()
        width = int(element.width)
        height = int(element.height)
        x = int(element.x)
        y = int(element.y)

        if element.anchor == "stretch_width":
            x = int(element.x)
            width = max(1, screen.width - int(element.x) * 2)
        elif element.anchor == "stretch_height":
            y = int(element.y)
            height = max(1, screen.height - int(element.y) * 2)
        elif element.anchor == "stretch":
            x = int(element.x)
            y = int(element.y)
            width = max(1, screen.width - int(element.x) * 2)
            height = max(1, screen.height - int(element.y) * 2)
        elif element.anchor in ("top_right", "right"):
            x = screen.width - width - int(element.x)
        elif element.anchor in ("bottom_left", "bottom"):
            y = screen.height - height - int(element.y)
        elif element.anchor == "bottom_right":
            x = screen.width - width - int(element.x)
            y = screen.height - height - int(element.y)
        elif element.anchor == "center":
            x = screen.centerx - width // 2 + int(element.x)
            y = screen.centery - height // 2 + int(element.y)

        return pygame.Rect(x, y, width, height)

    def set_focus(self, entity):
        self.focus_entity_id = getattr(entity, "id", None) if entity else None

    def focused(self):
        if not self.focus_entity_id:
            return None, None

        for entity, element in self.elements():
            if getattr(entity, "id", None) == self.focus_entity_id:
                return entity, element

        return None, None

    def elements(self):
        found = []

        for entity in getattr(self.game.world, "entities", []):
            element = entity.get_component("UIElement") if hasattr(entity, "get_component") else None

            if not element or not getattr(element, "enabled", True):
                continue

            if not getattr(entity, "enabled", True) or not getattr(entity, "visible", True):
                continue

            found.append((entity, element))

        return sorted(found, key=lambda item: item[1].sorting_order)

    def hit_test(self, pos):
        for entity, element in reversed(self.elements()):
            if self.element_rect(element).collidepoint(pos):
                return entity, element
        return None, None

    def handle_click(self, pos):
        entity, element = self.hit_test(pos)

        if not element or not element.interactable:
            return False

        self.focus_entity_id = getattr(entity, "id", None)

        visual_scripts = getattr(self.game, "visual_script_runtime", None)
        graph_name = getattr(element, "on_click_graph", None)

        if visual_scripts and graph_name:
            visual_scripts.execute_graph(entity, graph_name, "click")

        if hasattr(self.game, "console"):
            self.game.console.log(f"UI click: {getattr(entity, 'name', 'UIElement')}", "UI")

        return True

    def update(self, dt):
        mouse = pygame.mouse.get_pos()
        entity, element = self.hit_test(mouse)
        self.hover_entity_id = getattr(entity, "id", None) if element else None
        elements = self.elements()
        self.stats = {
            "elements": len(elements),
            "interactive": sum(1 for _, element in elements if element.interactable),
        }
