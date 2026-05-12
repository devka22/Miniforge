class EditorTools:
    """
    Acciones rápidas del editor.
    Mantiene funciones compactas para toolbar, consola e inspector.
    """

    def __init__(self, game):
        self.game = game

    def toggle_grid(self):
        self.game.editor_view_settings.toggle("show_grid")

    def toggle_gizmos(self):
        self.game.editor_view_settings.toggle("show_gizmos")

    def toggle_paths(self):
        self.game.editor_view_settings.toggle("show_paths")

    def toggle_names(self):
        self.game.editor_view_settings.toggle("show_names")

    def toggle_colliders(self):
        self.game.editor_view_settings.toggle("show_colliders")

    def toggle_chunks(self):
        self.game.editor_view_settings.toggle("show_chunks")

    def toggle_coordinates(self):
        self.game.editor_view_settings.toggle("show_tile_coordinates")

    def toggle_brush_preview(self):
        self.game.editor_view_settings.toggle("show_brush_preview")

    def toggle_selected_locked(self):
        for unit in self.game.selected_units:
            unit.locked = not getattr(unit, "locked", False)

        self.game.history.take_snapshot("Toggle Locked")
        self.game.console.log("Locked cambiado en selección", "EDITOR")

    def toggle_selected_visible(self):
        for unit in self.game.selected_units:
            unit.visible = not getattr(unit, "visible", True)

        self.game.history.take_snapshot("Toggle Visible")
        self.game.console.log("Visible cambiado en selección", "EDITOR")

    def set_brush_size(self, size):
        self.game.brush_size = max(1, min(5, int(size)))
        self.game.console.log(f"Brush size: {self.game.brush_size}", "EDITOR")