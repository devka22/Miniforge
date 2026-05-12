import json
import os

from engine.docking_panel import DockingPanel


class LayoutManager:
    """
    Guarda y carga layout de paneles movibles.
    """

    def __init__(self, game):
        self.game = game
        self.layout_file = "project/editor_layout.json"

        os.makedirs("project", exist_ok=True)

        self.panels = {}

        self.create_default_panels()
        self.load_layout()

    def create_default_panels(self):
        self.panels = {
            "hierarchy": DockingPanel("hierarchy", "Scene Hierarchy", 830, 70, 260, 170),
            "inspector": DockingPanel("inspector", "Inspector", 850, 250, 240, 390),
            "minimap": DockingPanel("minimap", "Minimap", 830, 500, 260, 130),
            "asset_preview": DockingPanel("asset_preview", "Asset Preview", 220, 365, 600, 120),
            "content_browser": DockingPanel("content_browser", "Content Browser", 220, 500, 600, 130),
            "console": DockingPanel("console", "Developer Console", 220, 635, 600, 95),
        }

        self.panels["console"].visible = False

    def get(self, panel_id):
        return self.panels.get(panel_id)

    def handle_event(self, event):
        # Reversed so panels drawn later are interacted first
        for panel in reversed(list(self.panels.values())):
            if panel.handle_event(event):
                return True

        return False

    def is_mouse_over_any_panel(self, pos):
        for panel in self.panels.values():
            if panel.is_mouse_over(pos):
                return True

        return False

    def save_layout(self):
        data = {}

        for panel_id, panel in self.panels.items():
            data[panel_id] = {
                "x": panel.rect.x,
                "y": panel.rect.y,
                "width": panel.rect.width,
                "height": panel.rect.height,
                "visible": panel.visible,
                "collapsed": panel.collapsed,
            }

        with open(self.layout_file, "w", encoding="utf-8") as file:
            json.dump(data, file, indent=4)

        self.game.console.log("Layout guardado", "ENGINE")

    def load_layout(self):
        if not os.path.exists(self.layout_file):
            return

        try:
            with open(self.layout_file, "r", encoding="utf-8") as file:
                data = json.load(file)

            for panel_id, panel_data in data.items():
                panel = self.panels.get(panel_id)

                if not panel:
                    continue

                panel.rect.x = panel_data.get("x", panel.rect.x)
                panel.rect.y = panel_data.get("y", panel.rect.y)
                panel.rect.width = panel_data.get("width", panel.rect.width)
                panel.rect.height = panel_data.get("height", panel.rect.height)
                panel.visible = panel_data.get("visible", panel.visible)
                panel.collapsed = panel_data.get("collapsed", panel.collapsed)

            self.fix_console_overlap()

        except Exception as error:
            self.game.console.log(f"No se pudo cargar layout: {error}", "WARNING")

    def fix_console_overlap(self):
        console = self.panels.get("console")
        browser = self.panels.get("content_browser")

        if not console or not browser:
            return

        if not console.rect.colliderect(browser.rect):
            return

        width, height = self.game.screen.get_size()
        console.rect.x = browser.rect.x
        console.rect.width = browser.rect.width
        console.rect.height = 95
        console.rect.y = min(
            max(browser.rect.bottom + 8, 64),
            max(64, height - console.rect.height - 42)
        )

        if console.rect.colliderect(browser.rect):
            console.collapsed = True
            console.rect.y = max(64, browser.rect.y - console.title_height - 8)

    def reset_layout(self):
        self.create_default_panels()
        self.save_layout()
        self.game.console.log("Layout reseteado", "ENGINE")

    def show_all_panels(self):
        for panel in self.panels.values():
            panel.visible = True
            panel.collapsed = False

        self.game.console.log("Todos los paneles visibles", "ENGINE")
