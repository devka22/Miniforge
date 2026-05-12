import pygame


class MenuBar:
    """
    Menu bar moderno.
    Los dropdowns se dibujan encima de la toolbar porque renderer lo dibuja al final.
    """

    def __init__(self, game):
        self.game = game
        self.height = 32
        self.font = pygame.font.SysFont(None, 18)
        self.active_menu = None
        self.menu_rects = {}

    def get_menus(self):
        return {
            "File": [
                ("New Scene", self.game.new_scene),
                ("Save Scene", self.game.save_scene),
                ("Load Scene", self.game.load_scene),
                ("Save Project", self.game.save_project),
                ("Validate Project", self.game.validate_project),
                ("Build Manifest", self.game.build_manifest),
                ("Export Build", self.game.export_build),
            ],
            "Edit": [
                ("Undo", self.game.undo),
                ("Redo", self.game.redo),
                ("Duplicate", self.game.duplicate_selected),
                ("Delete", self.game.delete_selected),
                ("Clear Selection", self.game.clear_selection),
            ],
            "Assets": [
                ("New Folder", self.game.open_new_folder_modal),
                ("New Script", self.game.open_new_script_modal),
                ("New Component", self.game.open_new_component_modal),
                ("New System", self.game.open_new_system_modal),
                ("New JSON", self.game.open_new_json_modal),
                ("New TXT", self.game.open_new_txt_modal),
                ("New Prefab", self.game.open_new_prefab_modal),
                ("Save Open File", self.game.script_editor.save),
                ("Import Sprite", self.game.import_sprite),
                ("Import Audio", self.game.import_audio),
                ("Import Data", self.game.import_data),
                ("Open Asset", self.game.file_browser.open_selected),
                ("Duplicate Asset", self.game.file_browser.duplicate_selected_asset),
                ("Rename Asset", self.game.open_rename_asset_modal),
                ("Delete Asset", self.game.file_browser.delete_selected_asset),
                ("Rename Folder", self.game.open_rename_folder_modal),
                ("Delete Folder", self.game.file_browser.delete_selected_folder),
                ("Import Settings", self.game.cycle_selected_asset_import_setting),
                ("Rebuild Dependencies", self.game.rebuild_asset_dependency_graph),
                ("Show Dependencies", self.game.print_selected_asset_dependencies),
                ("Refresh", self.game.refresh_project),
            ],
            "Entity": [
                ("Spawn Entity", self.game.spawn_unit),
                ("Create GameObject", self.game.create_game_object),
                ("Create Empty Child", self.game.create_empty_child),
                ("Duplicate Hierarchy", self.game.duplicate_selected_with_children),
                ("Delete Hierarchy", self.game.delete_selected_with_children),
                ("Parent To Active", self.game.parent_selection_to_active),
                ("Clear Parent", self.game.clear_selected_parent),
                ("Assign Sprite", self.game.assign_selected_sprite),
                ("Save Prefab", self.game.save_selected_prefab),
                ("Apply Prefab", self.game.prefab_workflow.apply_selected_to_prefab),
                ("Revert Prefab", self.game.prefab_workflow.revert_selected_prefab),
                ("Create Prefab Variant", self.game.create_prefab_variant),
                ("Nested Prefab Child", self.game.instantiate_nested_prefab),
            ],
            "View": [
                ("Scene/Game View", self.game.toggle_view_mode),
                ("Next Tab", self.game.cycle_editor_tab),
                ("Gizmo Mode", self.game.cycle_gizmo_mode),
                ("Grid Snapping", self.game.toggle_grid_snapping),
                ("Toggle Console", self.game.toggle_console),
                ("Console Filter", self.game.cycle_console_filter),
                ("Visual Input", self.game.visual_input_editor.toggle),
                ("Pause Profiler", self.game.toggle_profiler_pause),
                ("Show Grid", self.game.editor_tools.toggle_grid),
                ("Show Gizmos", self.game.editor_tools.toggle_gizmos),
                ("Show Paths", self.game.editor_tools.toggle_paths),
                ("Show Names", self.game.editor_tools.toggle_names),
                ("Show Colliders", self.game.editor_tools.toggle_colliders),
                ("Save Layout", self.game.save_editor_layout),
                ("Reset Layout", self.game.reset_editor_layout),
            ],
            "Run": [
                ("Play", self.game.play),
                ("Stop", self.game.stop),
                ("Pause", self.game.pause_play_mode),
                ("Restart Play", self.game.restart_play_mode),
                ("Build & Run", self.game.build_and_run),
                ("Build Profile", self.game.cycle_build_profile),
                ("Validate Scene", self.game.validate_scene),
                ("Validate Project", self.game.validate_project),
                ("Build Manifest", self.game.build_manifest),
                ("Export Build", self.game.export_build),
            ],
            "Create": [
                ("Add Animator", lambda: self.game.add_component_to_selected("Animator")),
                ("Add UI Element", lambda: self.game.add_component_to_selected("UIElement")),
                ("Add Visual Script", lambda: self.game.add_component_to_selected("VisualScript")),
                ("Add Rigidbody2D", lambda: self.game.add_component_to_selected("Rigidbody2D")),
                ("Playable Preset", lambda: self.game.apply_component_preset("Playable Unit")),
                ("Enemy AI Preset", lambda: self.game.apply_component_preset("Enemy AI")),
                ("TopDown Player Preset", lambda: self.game.apply_component_preset("TopDown Player")),
            ],
            "UI": [
                ("Create Label", self.game.create_ui_label),
                ("Create Button", self.game.create_ui_button),
                ("Create Progress Bar", self.game.create_ui_progress_bar),
                ("Create UI Example", self.game.create_example_ui_scene),
            ],
            "Visual": [
                ("Template Log And Move", lambda: self.game.add_visual_script_template("Log And Move")),
                ("Template Button Click", lambda: self.game.add_visual_script_template("Button Click")),
                ("Template Damage Self", lambda: self.game.add_visual_script_template("Damage Self")),
                ("Open Visual Input", self.game.visual_input_editor.toggle),
            ],
            "Templates": [
                ("Empty", self.game.create_template_empty),
                ("RTS", self.game.create_template_rts),
                ("Top Down", self.game.create_template_topdown),
                ("Platformer", self.game.create_template_platformer),
                ("Action RPG", self.game.create_template_action_rpg),
                ("Survival", self.game.create_template_survival),
                ("Example Action RPG", self.game.create_example_action_rpg),
            ],
            "RTS": [
                ("Stop", self.game.stop_selected_units),
                ("Hold", self.game.hold_selected_units),
                ("Cancel", self.game.command_system.cancel_units),
                ("Formation Square", lambda: self.game.set_rts_formation("square")),
                ("Formation Line", lambda: self.game.set_rts_formation("line")),
                ("Formation Column", lambda: self.game.set_rts_formation("column")),
                ("Formation Circle", lambda: self.game.set_rts_formation("circle")),
            ],
            "Tools": [
                ("Snap To Grid", self.game.snap_selected_to_grid),
                ("Snap Size 0.5", lambda: self.game.scene_view_tools.set_snap_size(0.5)),
                ("Snap Size 1", lambda: self.game.scene_view_tools.set_snap_size(1.0)),
                ("Align X", self.game.align_selected_x),
                ("Align Y", self.game.align_selected_y),
                ("Distribute X", self.game.distribute_selected_x),
                ("Distribute Y", self.game.distribute_selected_y),
                ("Focus Selected", self.game.center_camera_on_selection),
                ("Lock/Unlock", self.game.editor_tools.toggle_selected_locked),
                ("Hide/Show", self.game.editor_tools.toggle_selected_visible),
            ],
            "Plugins": [
                ("Scan Plugins", self.game.plugin_manager.scan),
                ("Hook Editor Start", lambda: self.game.plugin_hook("on_editor_start")),
                ("Hook Scene Saved", lambda: self.game.plugin_hook("on_scene_saved")),
                ("Hook Asset Imported", lambda: self.game.plugin_hook("on_asset_imported")),
            ],
        }

    def is_mouse_over(self, pos):
        x, y = pos

        if 0 <= y <= self.height:
            return True

        if self.active_menu:
            return self.dropdown_rect(self.active_menu).collidepoint(pos)

        return False

    def dropdown_rect(self, menu_name):
        rect = self.menu_rects.get(menu_name)

        if not rect:
            return pygame.Rect(0, 0, 0, 0)

        items = self.get_menus().get(menu_name, [])

        return pygame.Rect(
            rect.x,
            self.height,
            230,
            max(32, len(items) * 26 + 10)
        )

    def handle_event(self, event):
        if event.type != pygame.MOUSEBUTTONDOWN or event.button != 1:
            return False

        for name, rect in self.menu_rects.items():
            if rect.collidepoint(event.pos):
                self.active_menu = None if self.active_menu == name else name
                return True

        if self.active_menu:
            dropdown = self.dropdown_rect(self.active_menu)

            if dropdown.collidepoint(event.pos):
                items = self.get_menus().get(self.active_menu, [])
                local_y = event.pos[1] - dropdown.y - 6
                index = local_y // 26

                if 0 <= index < len(items):
                    _, callback = items[index]
                    self.active_menu = None
                    self.execute(callback)
                    return True

            self.active_menu = None
            return True

        return False

    def execute(self, callback):
        try:
            callback()
        except Exception as error:
            if hasattr(self.game, "console"):
                self.game.console.log(f"Menu action error: {error}", "ERROR")

    def draw(self, screen):
        width = screen.get_width()
        bar = pygame.Rect(0, 0, width, self.height)

        pygame.draw.rect(screen, (245, 246, 250), bar)
        pygame.draw.line(
            screen,
            (205, 208, 218),
            (0, self.height),
            (width, self.height)
        )

        logo = self.font.render("MiniForge", True, (35, 36, 42))
        screen.blit(logo, (12, 9))

        self.menu_rects.clear()

        x = 105

        for name in self.get_menus().keys():
            text = self.font.render(name, True, (45, 48, 56))
            rect = pygame.Rect(x - 8, 5, text.get_width() + 18, 22)

            if self.active_menu == name:
                pygame.draw.rect(screen, (215, 228, 255), rect, border_radius=6)

            elif rect.collidepoint(pygame.mouse.get_pos()):
                pygame.draw.rect(screen, (232, 235, 242), rect, border_radius=6)

            screen.blit(text, (x, 9))
            self.menu_rects[name] = rect

            x += rect.width + 8

        if self.active_menu:
            self.draw_dropdown(screen, self.active_menu)

    def draw_dropdown(self, screen, menu_name):
        dropdown = self.dropdown_rect(menu_name)
        items = self.get_menus().get(menu_name, [])

        shadow = dropdown.move(3, 3)

        pygame.draw.rect(screen, (0, 0, 0, 35), shadow, border_radius=10)
        pygame.draw.rect(screen, (252, 252, 254), dropdown, border_radius=10)
        pygame.draw.rect(screen, (190, 195, 208), dropdown, 1, border_radius=10)

        mouse = pygame.mouse.get_pos()

        for i, (label, _) in enumerate(items):
            row = pygame.Rect(
                dropdown.x + 6,
                dropdown.y + 6 + i * 26,
                dropdown.width - 12,
                24
            )

            if row.collidepoint(mouse):
                pygame.draw.rect(screen, (215, 228, 255), row, border_radius=6)

            img = self.font.render(label, True, (45, 48, 56))
            screen.blit(img, (row.x + 8, row.y + 6))
