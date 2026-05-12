class CommandPalette:
    """
    Command Palette estilo Unity/Unreal.
    Permite buscar y ejecutar acciones del editor sin navegar menús.
    """

    def __init__(self, game):
        self.game = game
        self.visible = False
        self.query = ""
        self.selected_index = 0

    def commands(self):
        return [
            ("Play", self.game.play),
            ("Stop", self.game.stop),
            ("Pause Play", self.game.pause_play_mode),
            ("Restart Play", self.game.restart_play_mode),
            ("Save Scene", self.game.save_scene),
            ("Load Scene", self.game.load_scene),
            ("New Scene", self.game.new_scene),
            ("Create GameObject", self.game.create_game_object),
            ("Create Empty Child", self.game.create_empty_child),
            ("Spawn Unit", self.game.spawn_unit),
            ("Duplicate Selected", self.game.duplicate_selected),
            ("Duplicate Hierarchy", self.game.duplicate_selected_with_children),
            ("Delete Selected", self.game.delete_selected),
            ("Delete Hierarchy", self.game.delete_selected_with_children),
            ("Parent To Active", self.game.parent_selection_to_active),
            ("Clear Parent", self.game.clear_selected_parent),
            ("Snap To Grid", self.game.snap_selected_to_grid),
            ("Align X", self.game.align_selected_x),
            ("Align Y", self.game.align_selected_y),
            ("Distribute X", self.game.distribute_selected_x),
            ("Distribute Y", self.game.distribute_selected_y),
            ("Gizmo Mode", self.game.cycle_gizmo_mode),
            ("Grid Snapping", self.game.toggle_grid_snapping),
            ("Validate Scene", self.game.validate_scene),
            ("Validate Project", self.game.validate_project),
            ("Build Manifest", self.game.build_manifest),
            ("Export Build", self.game.export_build),
            ("Build & Run", self.game.build_and_run),
            ("Toggle Console", self.game.toggle_console),
            ("Console Filter", self.game.cycle_console_filter),
            ("Visual Input Editor", self.game.visual_input_editor.toggle),
            ("Next Editor Tab", self.game.cycle_editor_tab),
            ("Reset Layout", self.game.reset_editor_layout),
            ("Refresh Project", self.game.refresh_project),
            ("Open Build Settings", lambda: self.game.open_settings_panel("Build")),
            ("Open Input Settings", lambda: self.game.open_settings_panel("Input")),
            ("Open Plugins", lambda: self.game.open_settings_panel("Plugins")),
            ("Create UI Label", self.game.create_ui_label),
            ("Create UI Button", self.game.create_ui_button),
            ("Create UI Progress Bar", self.game.create_ui_progress_bar),
            ("Create UI Example", self.game.create_example_ui_scene),
            ("Visual Script Log And Move", lambda: self.game.add_visual_script_template("Log And Move")),
            ("Visual Script Button Click", lambda: self.game.add_visual_script_template("Button Click")),
            ("Asset Rebuild Dependencies", self.game.rebuild_asset_dependency_graph),
            ("Asset Show Dependencies", self.game.print_selected_asset_dependencies),
            ("Asset Cycle Import Setting", self.game.cycle_selected_asset_import_setting),
            ("Create Action RPG Example", self.game.create_example_action_rpg),
            ("Create Survival Template", self.game.create_template_survival),
            ("Plugin Hook Editor Start", lambda: self.game.plugin_hook("on_editor_start")),
        ]

    def open(self):
        self.visible = True
        self.query = ""
        self.selected_index = 0

    def close(self):
        self.visible = False
        self.query = ""
        self.selected_index = 0

    def toggle(self):
        if self.visible:
            self.close()
        else:
            self.open()

    def filtered(self):
        query = self.query.strip().lower()
        commands = self.commands()

        if not query:
            return commands

        return [
            command for command in commands
            if query in command[0].lower()
        ]

    def move(self, delta):
        items = self.filtered()

        if not items:
            self.selected_index = 0
            return

        self.selected_index = (self.selected_index + delta) % len(items)

    def execute_selected(self):
        items = self.filtered()

        if not items:
            return False

        _, callback = items[self.selected_index]

        try:
            callback()
        except Exception as error:
            if hasattr(self.game, "console"):
                self.game.console.log(f"Command palette error: {error}", "ERROR")
            return False

        self.close()
        return True
