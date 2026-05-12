class EditorTabs:
    """
    Tabs ligeros para organizar paneles sin reemplazar el docking.
    """

    TABS = ["Scene", "Game", "Assets", "Debug"]

    VISIBLE_PANELS = {
        "Scene": {"hierarchy", "inspector", "minimap"},
        "Game": {"inspector", "minimap"},
        "Assets": {"asset_preview", "content_browser", "inspector"},
        "Debug": {"console", "hierarchy", "inspector"},
    }

    def __init__(self):
        self.active = "Scene"

    def cycle(self):
        index = self.TABS.index(self.active) if self.active in self.TABS else 0
        self.active = self.TABS[(index + 1) % len(self.TABS)]
        return self.active

    def set(self, tab):
        if tab in self.TABS:
            self.active = tab
            return True

        return False

    def panel_visible(self, panel_id):
        if panel_id == "console":
            return True

        return panel_id in self.VISIBLE_PANELS.get(self.active, set())
