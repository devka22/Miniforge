# input/input_handler.py

import time
import pygame


class InputHandler:
    """
    MiniForge InputHandler 0.6.0 Alpha.

    Maneja:
    - eventos generales
    - hotkeys
    - consola
    - editor de scripts
    - modal de creación/renombrado
    - navigator
    - content browser
    - scene hierarchy
    - inspector
    - minimap
    - input del mundo
    - cámara
    - project manager
    - layout manager
    - autosave/recover
    - herramientas del editor
    """

    def __init__(self, game):
        self.game = game

        # Selección en mundo
        self.dragging = False
        self.start_pos = (0, 0)
        self.current_pos = (0, 0)
        self.drag_threshold = 6
        self.mouse_down_over_ui = False

        # Content Browser
        self.last_browser_click_time = 0
        self.last_browser_click_index = -1
        self.dragging_asset_internal = False
        self.drag_start_asset_index = -1

        # Camera pan
        self.middle_pan = False
        self.middle_pan_last = (0, 0)
        self.gizmo_dragging = False
        self.gizmo_last_pos = (0, 0)

        # Double click general
        self.last_click_time = 0
        self.last_click_pos = (0, 0)

        # Estado general
        self.mouse_pos = (0, 0)
        self.prev_mouse_pos = (0, 0)
        self.mouse_delta = (0, 0)

    # =========================
    # MAIN EVENTS
    # =========================

    def handle_events(self):
        self.prev_mouse_pos = self.mouse_pos
        self.mouse_pos = pygame.mouse.get_pos()
        self.mouse_delta = (
            self.mouse_pos[0] - self.prev_mouse_pos[0],
            self.mouse_pos[1] - self.prev_mouse_pos[1],
        )

        self.handle_continuous_keyboard_camera()

        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                self.handle_quit()
                continue

            if self.handle_text_modes(event):
                continue

            if hasattr(self.game, "visual_input_editor"):
                if getattr(self.game.visual_input_editor, "visible", False):
                    if self.handle_visual_input_editor(event):
                        continue

            if hasattr(self.game, "command_palette"):
                if getattr(self.game.command_palette, "visible", False):
                    if self.handle_command_palette(event):
                        continue

            if self.safe_get(self.game, "console"):
                if getattr(self.game.console, "input_active", False):
                    if self.handle_console_input(event):
                        continue

            if self.safe_get(self.game, "script_editor"):
                if getattr(self.game.script_editor, "visible", False):
                    self.handle_script_editor(event)
                    continue

            if hasattr(self.game, "inspector_editor"):
                if getattr(self.game.inspector_editor, "editing", False):
                    if event.type == pygame.KEYDOWN:
                        if self.game.inspector_editor.handle_key(event):
                            continue

            if event.type == pygame.KEYDOWN:
                if self.handle_hotkeys(event):
                    continue

            if event.type == pygame.MOUSEWHEEL:
                self.handle_mouse_wheel(event)
                continue

            if self.handle_ui_events(event):
                continue

            if event.type == pygame.MOUSEBUTTONDOWN and event.button == 1:
                if hasattr(self.game, "ui_canvas") and self.game.ui_canvas.handle_click(event.pos):
                    continue

            if event.type == pygame.MOUSEBUTTONDOWN:
                self.handle_mouse_down(event)

            elif event.type == pygame.MOUSEBUTTONUP:
                self.handle_mouse_up(event)

            elif event.type == pygame.MOUSEMOTION:
                self.handle_mouse_motion(event)

            elif event.type == pygame.DROPFILE:
                self.handle_drop_file(event)

    def handle_quit(self):
        if hasattr(self.game, "layout_manager"):
            try:
                self.game.layout_manager.save_layout()
            except Exception as error:
                self.safe_log(f"No se pudo guardar layout: {error}", "WARNING")

        if hasattr(self.game, "project_manager"):
            try:
                if hasattr(self.game.project_manager, "save_project"):
                    self.game.project_manager.save_project()
            except Exception as error:
                self.safe_log(f"No se pudo guardar proyecto: {error}", "WARNING")

        if hasattr(self.game, "save_autosave"):
            try:
                self.game.save_autosave()
            except Exception as error:
                self.safe_log(f"No se pudo crear autosave al salir: {error}", "WARNING")

        self.game.running = False

    # =========================
    # TEXT MODES
    # =========================

    def handle_text_modes(self, event):
        if event.type != pygame.KEYDOWN:
            return False

        if hasattr(self.game, "create_asset_modal"):
            if self.game.create_asset_modal.visible:
                return self.game.create_asset_modal.handle_key(event)

        if getattr(self.game, "navigator_search_active", False):
            return self.handle_navigator_search_key(event)

        if hasattr(self.game, "scene_hierarchy"):
            if getattr(self.game.scene_hierarchy, "search_active", False):
                return self.game.scene_hierarchy.handle_search_key(event)

        if getattr(self.game, "settings_editing_key", None):
            return self.handle_settings_edit_key(event)

        return False

    def key_name_from_event(self, event):
        if event.type == pygame.MOUSEBUTTONDOWN:
            if event.button == 1:
                return "mouse1"
            if event.button == 3:
                return "mouse2"
            return f"mouse{event.button}"

        if event.type != pygame.KEYDOWN:
            return None

        name = pygame.key.name(event.key)
        return name.replace(" ", "_")

    def handle_visual_input_editor(self, event):
        editor = self.game.visual_input_editor

        if editor.capture_mode:
            key_name = self.key_name_from_event(event)

            if key_name:
                editor.add_binding(key_name)
                return True

        if event.type == pygame.KEYDOWN:
            if event.key == pygame.K_ESCAPE:
                editor.visible = False
                editor.capture_mode = False
                return True

            if event.key == pygame.K_UP:
                editor.scroll = max(0, editor.scroll - 1)
                return True

            if event.key == pygame.K_DOWN:
                editor.scroll += 1
                return True

        if event.type != pygame.MOUSEBUTTONDOWN or event.button != 1:
            return True

        screen_w, screen_h = self.game.screen.get_size()
        rect = pygame.Rect(screen_w // 2 - 330, 92, 660, min(520, screen_h - 140))

        if not rect.collidepoint(event.pos):
            return True

        x, y = event.pos

        if pygame.Rect(rect.right - 36, rect.y + 12, 24, 22).collidepoint(x, y):
            editor.visible = False
            return True

        list_rect = pygame.Rect(rect.x + 16, rect.y + 52, 210, rect.height - 72)
        detail_rect = pygame.Rect(rect.x + 240, rect.y + 52, rect.width - 256, rect.height - 72)

        actions = editor.actions()

        for i, action in enumerate(actions[editor.scroll:editor.scroll + 16]):
            row = pygame.Rect(list_rect.x + 8, list_rect.y + 8 + i * 24, list_rect.width - 16, 22)

            if row.collidepoint(x, y):
                editor.select(action)
                return True

        button_y = detail_rect.y + 14 + 36 + 24 + len(self.game.input_map.bindings.get(editor.selected_action, [])) * 30 + 8

        if pygame.Rect(detail_rect.x + 14, button_y, 130, 28).collidepoint(x, y):
            editor.start_capture()
            return True

        if pygame.Rect(detail_rect.x + 154, button_y, 130, 28).collidepoint(x, y):
            editor.remove_last_binding()
            return True

        if pygame.Rect(detail_rect.x + 294, button_y, 90, 28).collidepoint(x, y):
            editor.create_next_action()
            return True

        return True

    def handle_command_palette(self, event):
        palette = self.game.command_palette

        if event.type != pygame.KEYDOWN:
            return True

        if event.key == pygame.K_ESCAPE:
            palette.close()
            return True

        if event.key == pygame.K_RETURN:
            palette.execute_selected()
            return True

        if event.key == pygame.K_UP:
            palette.move(-1)
            return True

        if event.key == pygame.K_DOWN:
            palette.move(1)
            return True

        if event.key == pygame.K_BACKSPACE:
            palette.query = palette.query[:-1]
            palette.selected_index = 0
            return True

        if event.unicode and event.unicode.isprintable():
            palette.query += event.unicode
            palette.selected_index = 0
            return True

        return True

    def handle_navigator_search_key(self, event):
        if event.key == pygame.K_ESCAPE:
            self.game.clear_navigator_search()
            return True

        if event.key == pygame.K_RETURN:
            self.game.navigator_search_active = False
            return True

        if event.key == pygame.K_BACKSPACE:
            self.game.navigator_search_text = self.game.navigator_search_text[:-1]
            self.game.navigator_scroll = 0
            return True

        if event.unicode and event.unicode.isprintable():
            self.game.navigator_search_text += event.unicode
            self.game.navigator_scroll = 0
            return True

        return True

    def handle_settings_edit_key(self, event):
        if event.key == pygame.K_ESCAPE:
            self.game.cancel_settings_edit()
            return True

        if event.key == pygame.K_RETURN:
            self.game.commit_settings_edit()
            return True

        if event.key == pygame.K_BACKSPACE:
            self.game.settings_edit_buffer = self.game.settings_edit_buffer[:-1]
            return True

        if event.unicode and event.unicode.isprintable():
            self.game.settings_edit_buffer += event.unicode
            return True

        return True

    # =========================
    # CONSOLE
    # =========================

    def handle_console_input(self, event):
        if event.type != pygame.KEYDOWN:
            return False

        if event.key == pygame.K_ESCAPE:
            self.game.console.input_active = False
            self.game.console.command_buffer = ""
            return True

        if event.key == pygame.K_RETURN:
            self.game.console.submit_command()
            return True

        if event.key == pygame.K_BACKSPACE:
            self.game.console.command_buffer = self.game.console.command_buffer[:-1]
            return True

        if event.unicode and event.unicode.isprintable():
            self.game.console.command_buffer += event.unicode
            return True

        return False

    # =========================
    # HOTKEYS
    # =========================

    def command_pressed(self):
        keys = pygame.key.get_pressed()

        return (
            keys[pygame.K_LMETA]
            or keys[pygame.K_RMETA]
            or keys[pygame.K_LCTRL]
            or keys[pygame.K_RCTRL]
        )

    def shift_pressed(self):
        keys = pygame.key.get_pressed()
        return keys[pygame.K_LSHIFT] or keys[pygame.K_RSHIFT]

    def alt_pressed(self):
        keys = pygame.key.get_pressed()
        return keys[pygame.K_LALT] or keys[pygame.K_RALT]

    def handle_hotkeys(self, event):
        command = self.command_pressed()
        shift = self.shift_pressed()

        number_keys = {
            pygame.K_1: 1,
            pygame.K_2: 2,
            pygame.K_3: 3,
            pygame.K_4: 4,
            pygame.K_5: 5,
            pygame.K_6: 6,
            pygame.K_7: 7,
            pygame.K_8: 8,
            pygame.K_9: 9,
        }

        # =========================
        # ESC SAFE CLOSE ORDER
        # =========================

        if event.key == pygame.K_ESCAPE:
            if hasattr(self.game, "create_asset_modal"):
                if self.game.create_asset_modal.visible:
                    self.game.create_asset_modal.close()
                    return True

            if hasattr(self.game, "file_browser"):
                if getattr(self.game.file_browser, "context_menu_open", False):
                    self.game.file_browser.close_context_menu()
                    return True

            if getattr(self.game, "navigator_search_active", False):
                self.game.clear_navigator_search()
                return True

            if getattr(self.game, "active_settings_panel", None):
                self.game.close_settings_panel()
                return True

            if hasattr(self.game, "script_editor"):
                if getattr(self.game.script_editor, "visible", False):
                    self.game.script_editor.toggle()
                    return True

            self.game.clear_selection()
            return True

        # =========================
        # COMMAND / CTRL SHORTCUTS
        # =========================

        if command and event.key == pygame.K_s:
            if hasattr(self.game, "save_scene"):
                self.game.save_scene()
            return True

        if command and event.key == pygame.K_k:
            if hasattr(self.game, "command_palette"):
                self.game.command_palette.toggle()
            return True

        if command and event.key == pygame.K_z:
            if shift:
                if hasattr(self.game, "redo"):
                    self.game.redo()
            else:
                if hasattr(self.game, "undo"):
                    self.game.undo()
            return True

        if command and event.key == pygame.K_y:
            if hasattr(self.game, "redo"):
                self.game.redo()
            return True

        if command and event.key == pygame.K_l:
            if hasattr(self.game, "save_editor_layout"):
                self.game.save_editor_layout()
            return True

        if command and event.key == pygame.K_a:
            if hasattr(self.game, "selection_manager"):
                self.game.selection_manager.select_all()
            return True

        if command and event.key == pygame.K_f:
            self.game.navigator_search_active = True
            self.game.navigator_search_text = ""
            self.game.navigator_scroll = 0
            return True

        if command and event.key == pygame.K_n:
            if hasattr(self.game, "new_scene"):
                self.game.new_scene()
            return True

        if command and event.key == pygame.K_b:
            if hasattr(self.game, "build_manifest"):
                self.game.build_manifest()
            return True

        if command and event.key == pygame.K_r:
            if hasattr(self.game, "refresh_project"):
                self.game.refresh_project()
            return True

        if command and event.key == pygame.K_o:
            if hasattr(self.game, "open_project"):
                self.game.open_project()
            elif hasattr(self.game, "load_scene"):
                self.game.load_scene()
            return True

        if command and event.key == pygame.K_d:
            if hasattr(self.game, "duplicate_selected"):
                if shift and hasattr(self.game, "duplicate_selected_with_children"):
                    self.game.duplicate_selected_with_children()
                else:
                    self.game.duplicate_selected()
            return True

        if command and event.key == pygame.K_g:
            if hasattr(self.game, "snap_selected_to_grid"):
                self.game.snap_selected_to_grid()
            return True

        if command and event.key == pygame.K_e:
            if hasattr(self.game, "create_empty_child"):
                self.game.create_empty_child()
            return True

        if command and event.key == pygame.K_c:
            if hasattr(self.game, "copy_selected"):
                self.game.copy_selected()
            return True

        if command and event.key == pygame.K_v:
            if hasattr(self.game, "paste_selected"):
                self.game.paste_selected()
            elif hasattr(self.game, "paste_object"):
                self.game.paste_object()
            return True

        # =========================
        # CONTROL GROUPS / TOOLS
        # =========================

        if event.key in number_keys:
            number = number_keys[event.key]

            if command:
                if hasattr(self.game, "assign_control_group"):
                    self.game.assign_control_group(number)
                return True

            if hasattr(self.game, "select_control_group"):
                if self.game.select_control_group(number):
                    return True

            tool_map = {
                1: "Select",
                2: "Move",
                3: "Entity",
                4: "Tile",
                5: "Obstacle",
                6: "Erase",
            }

            if number in tool_map:
                self.game.active_tool = tool_map[number]
                self.safe_log(f"Herramienta activa: {tool_map[number]}", "EDITOR")
                return True

        # =========================
        # FUNCTION KEYS
        # =========================

        if event.key == pygame.K_F1:
            if hasattr(self.game, "console"):
                self.game.console.toggle()
            return True

        if event.key == pygame.K_BACKQUOTE:
            if hasattr(self.game, "console"):
                self.game.console.toggle_input()
            return True

        if event.key == pygame.K_F2:
            if hasattr(self.game, "script_editor"):
                self.game.script_editor.toggle()
            return True

        if event.key == pygame.K_F3:
            if hasattr(self.game, "editor_tools"):
                self.game.editor_tools.toggle_debug()
            elif hasattr(self.game, "renderer"):
                self.game.renderer.show_debug = not self.game.renderer.show_debug
            return True

        if event.key == pygame.K_F5:
            if hasattr(self.game, "toggle_mode"):
                self.game.toggle_mode()
            return True

        if event.key == pygame.K_F11:
            if hasattr(self.game, "pause_play_mode"):
                self.game.pause_play_mode()
            return True

        if event.key == pygame.K_F12:
            if hasattr(self.game, "restart_play_mode"):
                self.game.restart_play_mode()
            return True

        if event.key == pygame.K_F6:
            if hasattr(self.game, "toggle_view_mode"):
                self.game.toggle_view_mode()
            return True

        if event.key == pygame.K_F7:
            if hasattr(self.game, "validate_scene"):
                self.game.validate_scene()
            return True

        if event.key == pygame.K_F8:
            if hasattr(self.game, "build_manifest"):
                self.game.build_manifest()
            return True

        if event.key == pygame.K_F9:
            if hasattr(self.game, "recover_autosave"):
                self.game.recover_autosave()
            return True

        if event.key == pygame.K_F10:
            if hasattr(self.game, "validate_project"):
                self.game.validate_project()
            return True

        # =========================
        # EDITOR ACTIONS
        # =========================

        if event.key in [pygame.K_DELETE, pygame.K_BACKSPACE]:
            if hasattr(self.game, "delete_selected"):
                self.game.delete_selected()
            return True

        if event.key == pygame.K_RETURN:
            if hasattr(self.game, "file_browser"):
                self.game.file_browser.open_selected()
            return True

        if event.key == pygame.K_TAB:
            if hasattr(self.game, "file_browser"):
                self.game.file_browser.cycle_filter()
            return True

        if event.key == pygame.K_f:
            if hasattr(self.game, "center_camera_on_selection"):
                self.game.center_camera_on_selection()
            return True

        if event.key == pygame.K_s:
            if hasattr(self.game, "stop_selected_units"):
                self.game.stop_selected_units()
            return True

        if event.key == pygame.K_h:
            if hasattr(self.game, "hold_selected_units"):
                self.game.hold_selected_units()
            return True

        if event.key == pygame.K_p:
            if hasattr(self.game, "patrol_selected_units_to_screen"):
                self.game.patrol_selected_units_to_screen(pygame.mouse.get_pos())
            return True

        if event.key == pygame.K_c:
            if hasattr(self.game, "clear_selected_paths"):
                self.game.clear_selected_paths()
            return True

        if event.key == pygame.K_e and shift:
            if hasattr(self.game, "toggle_selected_enabled"):
                self.game.toggle_selected_enabled()
            return True

        if event.key == pygame.K_r and shift:
            if hasattr(self.game, "reset_selected_transform"):
                self.game.reset_selected_transform()
            return True

        if event.key == pygame.K_g:
            if hasattr(self.game, "editor_tools"):
                self.game.editor_tools.toggle_grid()
            elif hasattr(self.game, "renderer"):
                self.game.renderer.show_grid = not self.game.renderer.show_grid
            return True

        if event.key == pygame.K_v:
            if hasattr(self.game, "editor_tools"):
                self.game.editor_tools.toggle_gizmos()
            return True

        if event.key == pygame.K_w:
            if hasattr(self.game, "scene_view_tools"):
                self.game.scene_view_tools.gizmo_mode = "Move"
                self.game.active_tool = "Move"
                self.safe_log("Gizmo: Move", "EDITOR")
            return True

        if event.key == pygame.K_e:
            if hasattr(self.game, "scene_view_tools"):
                self.game.scene_view_tools.gizmo_mode = "Rotate"
                self.game.active_tool = "Rotate"
                self.safe_log("Gizmo: Rotate", "EDITOR")
            return True

        if event.key == pygame.K_r:
            if hasattr(self.game, "scene_view_tools"):
                self.game.scene_view_tools.gizmo_mode = "Scale"
                self.game.active_tool = "Scale"
                self.safe_log("Gizmo: Scale", "EDITOR")
            return True

        return False

    # =========================
    # CONTINUOUS CAMERA
    # =========================

    def handle_continuous_keyboard_camera(self):
        if not hasattr(self.game, "camera"):
            return

        if self.safe_get(self.game, "console"):
            if getattr(self.game.console, "input_active", False):
                return

        if self.safe_get(self.game, "script_editor"):
            if getattr(self.game.script_editor, "visible", False):
                return

        if hasattr(self.game, "create_asset_modal"):
            if self.game.create_asset_modal.visible:
                return

        keys = pygame.key.get_pressed()

        speed = 10 / max(0.2, getattr(self.game.camera, "zoom", 1.0))

        if keys[pygame.K_LSHIFT] or keys[pygame.K_RSHIFT]:
            speed *= 2

        moved = False

        if keys[pygame.K_w] or keys[pygame.K_UP]:
            self.game.camera.y -= speed
            moved = True

        if keys[pygame.K_s] or keys[pygame.K_DOWN]:
            self.game.camera.y += speed
            moved = True

        if keys[pygame.K_a] or keys[pygame.K_LEFT]:
            self.game.camera.x -= speed
            moved = True

        if keys[pygame.K_d] or keys[pygame.K_RIGHT]:
            self.game.camera.x += speed
            moved = True

        if moved and hasattr(self.game.camera, "clamp"):
            try:
                self.game.camera.clamp()
            except Exception:
                pass

    # =========================
    # SCROLL
    # =========================

    def handle_mouse_wheel(self, event):
        mx, my = pygame.mouse.get_pos()

        if hasattr(self.game, "is_mouse_over_left_panel"):
            if self.game.is_mouse_over_left_panel((mx, my)):
                self.game.navigator_scroll -= event.y * 32
                self.game.navigator_scroll = max(
                    0,
                    min(self.game.navigator_max_scroll, self.game.navigator_scroll)
                )
                return True

        if hasattr(self.game, "layout_manager"):
            if self.game.layout_manager.is_mouse_over_any_panel((mx, my)):
                if self.mouse_over_content_browser():
                    if self.mouse_over_content_browser_folders():
                        if event.y > 0:
                            self.game.file_browser.folder_scroll_up()
                        else:
                            self.game.file_browser.folder_scroll_down()
                    else:
                        if event.y > 0:
                            self.game.file_browser.scroll_up()
                        else:
                            self.game.file_browser.scroll_down()

                    return True

                if self.mouse_over_scene_hierarchy():
                    if event.y > 0:
                        self.game.scene_hierarchy.scroll_up()
                    else:
                        self.game.scene_hierarchy.scroll_down()

                    return True

                return True

        if hasattr(self.game, "camera"):
            if hasattr(self.game.camera, "zoom_by"):
                self.game.camera.zoom_by(event.y * 0.08)

        return True

    # =========================
    # PANEL AREAS
    # =========================

    def get_panel_content_rect(self, panel_id, fallback):
        if hasattr(self.game, "layout_manager"):
            panel = self.game.layout_manager.get(panel_id)

            if panel and panel.visible and not panel.collapsed:
                return panel.content_rect()

        return fallback

    def mouse_over_content_browser(self):
        mx, my = pygame.mouse.get_pos()

        rect = self.get_panel_content_rect(
            "content_browser",
            pygame.Rect(220, 500, 600, 130)
        )

        return rect.collidepoint(mx, my)

    def mouse_over_content_browser_folders(self):
        mx, my = pygame.mouse.get_pos()

        rect = self.get_panel_content_rect(
            "content_browser",
            pygame.Rect(220, 500, 600, 130)
        )

        if not hasattr(self.game, "file_browser"):
            return False

        folder_width = 170 if self.game.file_browser.tree_view else 0

        folder_rect = pygame.Rect(
            rect.x + 8,
            rect.y + 56,
            folder_width,
            rect.height - 64
        )

        return folder_rect.collidepoint(mx, my)

    def mouse_over_scene_hierarchy(self):
        mx, my = pygame.mouse.get_pos()

        rect = self.get_panel_content_rect(
            "hierarchy",
            pygame.Rect(830, 70, 260, 170)
        )

        return rect.collidepoint(mx, my)

    def mouse_over_minimap(self):
        mx, my = pygame.mouse.get_pos()

        rect = self.get_panel_content_rect(
            "minimap",
            pygame.Rect(830, 500, 260, 130)
        )

        return rect.collidepoint(mx, my)

    # =========================
    # SCRIPT EDITOR
    # =========================

    def handle_script_editor(self, event):
        editor = self.game.script_editor

        if event.type == pygame.MOUSEWHEEL:
            if event.y > 0:
                editor.scroll_up()
            else:
                editor.scroll_down()
            return

        if event.type == pygame.MOUSEBUTTONDOWN and event.button == 1:
            width, height = self.game.screen.get_size()
            rect = pygame.Rect(230, 80, width - 280, height - 180)
            button_x = rect.right - 252

            for label, callback in [
                ("New", lambda: editor.create_new_script("NewScript")),
                ("Save", editor.save),
                ("Run", editor.run_active),
                ("Reload", editor.reload_scripts),
            ]:
                button_rect = pygame.Rect(button_x, rect.y + 7, 56, 22)

                if button_rect.collidepoint(event.pos):
                    callback()
                    return

                button_x += 62

            return

        if event.type != pygame.KEYDOWN:
            return

        keys = pygame.key.get_pressed()

        command = (
            keys[pygame.K_LMETA]
            or keys[pygame.K_RMETA]
            or keys[pygame.K_LCTRL]
            or keys[pygame.K_RCTRL]
        )

        if event.key == pygame.K_ESCAPE:
            editor.toggle()

        elif event.key == pygame.K_s and command:
            if keys[pygame.K_LSHIFT] or keys[pygame.K_RSHIFT]:
                editor.save_all()
            else:
                editor.save()

        elif event.key == pygame.K_RETURN and command:
            editor.attach_to_selected()

        elif event.key == pygame.K_SPACE and (
            keys[pygame.K_LCTRL] or keys[pygame.K_RCTRL]
        ):
            editor.autocomplete()

        elif event.key == pygame.K_F2:
            editor.insert_snippet("move_right")

        elif event.key == pygame.K_F3:
            editor.insert_snippet("log")

        elif event.key == pygame.K_F4:
            editor.toggle_errors()

        elif event.key == pygame.K_F5:
            editor.toggle_symbols()

        elif event.key == pygame.K_F6:
            editor.reload_scripts()

        elif event.key == pygame.K_F7:
            editor.run_active()

        elif event.key == pygame.K_TAB and command:
            editor.switch_tab(1)

        elif event.key == pygame.K_w and command:
            editor.close_tab()

        elif event.key == pygame.K_RETURN:
            editor.new_line()

        elif event.key == pygame.K_BACKSPACE:
            editor.backspace()

        elif event.key == pygame.K_LEFT:
            editor.move_left()

        elif event.key == pygame.K_RIGHT:
            editor.move_right()

        elif event.key == pygame.K_UP:
            editor.move_up()

        elif event.key == pygame.K_DOWN:
            editor.move_down()

        elif event.key == pygame.K_TAB:
            editor.insert_char("    ")

        else:
            if event.unicode and event.unicode.isprintable():
                editor.insert_char(event.unicode)

    # =========================
    # UI EVENTS
    # =========================

    def handle_ui_events(self, event):
        if self.handle_create_asset_modal_click(event):
            return True

        if hasattr(self.game, "menu_bar"):
            if self.game.menu_bar.handle_event(event):
                return True

        if hasattr(self.game, "toolbar"):
            if self.game.toolbar.handle_event(event):
                return True

        if self.handle_editor_tabs_click(event):
            return True

        if self.handle_navigator_click(event):
            return True

        if hasattr(self.game, "layout_manager"):
            if self.game.layout_manager.handle_event(event):
                return True

        if self.handle_content_browser_event(event):
            return True

        if self.handle_hierarchy_click(event):
            return True

        if self.handle_settings_panel_click(event):
            return True

        if self.handle_inspector_section_click(event):
            return True

        if self.handle_minimap_click(event):
            return True

        if self.handle_inspector_click(event):
            return True

        return False

    def handle_editor_tabs_click(self, event):
        if event.type != pygame.MOUSEBUTTONDOWN or event.button != 1:
            return False

        if not hasattr(self.game, "editor_tabs"):
            return False

        x = 220
        y = 66

        for tab in self.game.editor_tabs.TABS:
            rect = pygame.Rect(x, y, 74, 22)

            if rect.collidepoint(event.pos):
                self.game.editor_tabs.set(tab)
                self.game.active_editor_tab = tab
                self.game.console.log(f"Editor tab: {tab}", "EDITOR")
                return True

            x += 80

        return False

    # =========================
    # CREATE / RENAME MODAL
    # =========================

    def handle_create_asset_modal_click(self, event):
        modal = getattr(self.game, "create_asset_modal", None)

        if not modal or not modal.visible:
            return False

        if event.type != pygame.MOUSEBUTTONDOWN or event.button != 1:
            return True

        screen_w, screen_h = self.game.screen.get_size()

        rect = pygame.Rect(
            screen_w // 2 - 190,
            screen_h // 2 - 90,
            380,
            180
        )

        create_rect = pygame.Rect(rect.right - 190, rect.bottom - 42, 80, 26)
        cancel_rect = pygame.Rect(rect.right - 100, rect.bottom - 42, 80, 26)
        input_rect = pygame.Rect(rect.x + 72, rect.y + 54, rect.width - 95, 30)

        if create_rect.collidepoint(event.pos):
            modal.confirm()
            return True

        if cancel_rect.collidepoint(event.pos):
            modal.close()
            return True

        if input_rect.collidepoint(event.pos):
            return True

        return True

    # =========================
    # NAVIGATOR
    # =========================

    def handle_navigator_click(self, event):
        if event.type != pygame.MOUSEBUTTONDOWN or event.button != 1:
            return False

        x, y = event.pos

        if not hasattr(self.game, "is_mouse_over_left_panel"):
            return False

        if not self.game.is_mouse_over_left_panel((x, y)):
            return False

        panel_x = 8
        panel_y = 72
        scroll = getattr(self.game, "navigator_scroll", 0)

        search_rect = pygame.Rect(panel_x + 12, panel_y + 52, 166, 24)

        if search_rect.collidepoint(x, y):
            self.game.navigator_search_active = True
            self.game.navigator_search_text = ""
            return True

        if not hasattr(self.game, "get_navigator_actions"):
            return True

        actions = self.game.get_navigator_actions()
        cursor_y = 86

        for section_name, items in actions.items():
            header_rect = pygame.Rect(
                panel_x + 10,
                panel_y + cursor_y - scroll,
                170,
                26
            )

            if header_rect.collidepoint(x, y):
                self.game.toggle_navigator_section(section_name)
                return True

            cursor_y += 32

            opened = self.game.navigator_sections_open.get(section_name, False)
            force_open = bool(self.game.navigator_search_text.strip())

            if opened or force_open:
                for _, callback in items:
                    item_rect = pygame.Rect(
                        panel_x + 18,
                        panel_y + cursor_y - scroll,
                        156,
                        24
                    )

                    if item_rect.collidepoint(x, y):
                        callback()
                        return True

                    cursor_y += 28

            cursor_y += 4

        return True

    # =========================
    # CONTENT BROWSER
    # =========================

    def handle_content_browser_event(self, event):
        if not hasattr(self.game, "file_browser"):
            return False

        if self.handle_content_context_menu(event):
            return True

        if event.type == pygame.MOUSEBUTTONDOWN:
            return self.handle_content_browser_mouse_down(event)

        if event.type == pygame.MOUSEBUTTONUP:
            return self.handle_content_browser_mouse_up(event)

        if event.type == pygame.MOUSEMOTION:
            return self.handle_content_browser_motion(event)

        return False

    def get_context_menu_draw_rect(self):
        fb = self.game.file_browser

        x, y = fb.context_menu_pos
        item_h = 24
        width = 180
        height = len(fb.context_menu_items) * item_h

        screen_w, screen_h = self.game.screen.get_size()

        if x + width > screen_w:
            x = screen_w - width - 8

        if y + height > screen_h:
            y = screen_h - height - 8

        return pygame.Rect(x, y, width, height), item_h

    def handle_content_context_menu(self, event):
        fb = self.game.file_browser

        if not hasattr(fb, "context_menu_open"):
            return False

        if event.type == pygame.MOUSEBUTTONDOWN and event.button == 1:
            if fb.context_menu_open:
                menu_rect, item_h = self.get_context_menu_draw_rect()
                mx, my = event.pos

                for i, (_, action) in enumerate(fb.context_menu_items):
                    row = pygame.Rect(
                        menu_rect.x,
                        menu_rect.y + i * item_h,
                        menu_rect.width,
                        item_h
                    )

                    if row.collidepoint(mx, my):
                        fb.execute_context_action(action)
                        return True

                fb.close_context_menu()
                return True

        if event.type == pygame.MOUSEBUTTONDOWN and event.button == 3:
            rect = self.get_panel_content_rect(
                "content_browser",
                pygame.Rect(220, 500, 600, 130)
            )

            if not rect.collidepoint(event.pos):
                fb.close_context_menu()
                return False

            x, y = event.pos
            folder_width = 170 if fb.tree_view else 0

            folder_rect = pygame.Rect(
                rect.x + 8,
                rect.y + 56,
                folder_width,
                rect.height - 64
            )

            asset_rect = pygame.Rect(
                rect.x + 12 + folder_width,
                rect.y + 56,
                rect.width - folder_width - 20,
                rect.height - 64
            )

            if folder_rect.collidepoint(x, y):
                index = (y - folder_rect.y) // 20
                selected = fb.select_folder_by_index(index)

                if selected:
                    fb.open_context_menu(event.pos, "folder")
                else:
                    fb.open_context_menu(event.pos, "empty")

                return True

            if asset_rect.collidepoint(x, y):
                index = (y - asset_rect.y) // 20
                selected = fb.select_asset_by_index(index)

                if selected:
                    fb.open_context_menu(event.pos, "asset")
                else:
                    fb.open_context_menu(event.pos, "empty")

                return True

            fb.open_context_menu(event.pos, "empty")
            return True

        return False

    def handle_content_browser_mouse_down(self, event):
        if event.button != 1:
            return False

        fb = self.game.file_browser

        if getattr(fb, "context_menu_open", False):
            return True

        rect = self.get_panel_content_rect(
            "content_browser",
            pygame.Rect(220, 500, 600, 130)
        )

        x, y = event.pos

        if not rect.collidepoint(x, y):
            return False

        if self.handle_browser_quick_buttons(event, rect):
            return True

        folder_width = 170 if fb.tree_view else 0

        folder_rect = pygame.Rect(
            rect.x + 8,
            rect.y + 56,
            folder_width,
            rect.height - 64
        )

        asset_rect = pygame.Rect(
            rect.x + 12 + folder_width,
            rect.y + 56,
            rect.width - folder_width - 20,
            rect.height - 64
        )

        if folder_rect.collidepoint(x, y):
            index = (y - folder_rect.y) // 20

            if fb.select_folder_by_index(index):
                return True

        if asset_rect.collidepoint(x, y):
            index = (y - asset_rect.y) // 20

            if 0 <= index < fb.max_visible:
                selected = fb.select_asset_by_index(index)

                if not selected:
                    return True

                now = time.time()

                if (
                    self.last_browser_click_index == index
                    and now - self.last_browser_click_time < 0.35
                ):
                    fb.open_selected()

                self.last_browser_click_time = now
                self.last_browser_click_index = index

                self.dragging_asset_internal = True
                self.drag_start_asset_index = index

                if hasattr(fb, "start_drag_selected"):
                    fb.start_drag_selected()

                return True

        return True

    def handle_browser_quick_buttons(self, event, rect):
        if event.button != 1:
            return False

        x, y = event.pos

        buttons = [
            ("script", pygame.Rect(rect.x + 10, rect.y + 30, 66, 22)),
            ("folder", pygame.Rect(rect.x + 82, rect.y + 30, 66, 22)),
            ("scene", pygame.Rect(rect.x + 158, rect.y + 30, 66, 22)),
            ("prefab", pygame.Rect(rect.x + 230, rect.y + 30, 66, 22)),
            ("import", pygame.Rect(rect.x + 310, rect.y + 30, 66, 22)),
            ("refresh", pygame.Rect(rect.x + 382, rect.y + 30, 72, 22)),
        ]

        for action, button_rect in buttons:
            if button_rect.collidepoint(x, y):
                if action == "script":
                    self.game.open_create_modal("create_script")
                elif action == "folder":
                    self.game.open_create_modal("create_folder")
                elif action == "scene":
                    self.game.open_create_modal("create_scene")
                elif action == "prefab":
                    self.game.open_create_modal("create_prefab")
                elif action == "import":
                    self.game.import_sprite()
                elif action == "refresh":
                    self.game.refresh_project()

                return True

        return False

    def handle_content_browser_motion(self, event):
        if not self.dragging_asset_internal:
            return False

        fb = self.game.file_browser

        rect = self.get_panel_content_rect(
            "content_browser",
            pygame.Rect(220, 500, 600, 130)
        )

        if not rect.collidepoint(event.pos):
            return False

        folder_width = 170 if fb.tree_view else 0

        folder_rect = pygame.Rect(
            rect.x + 8,
            rect.y + 56,
            folder_width,
            rect.height - 64
        )

        if folder_rect.collidepoint(event.pos):
            index = (event.pos[1] - folder_rect.y) // 20

            if hasattr(fb, "set_drag_hover_folder_by_index"):
                fb.set_drag_hover_folder_by_index(index)

        return True

    def handle_content_browser_mouse_up(self, event):
        if not self.dragging_asset_internal:
            return False

        self.dragging_asset_internal = False

        if hasattr(self.game, "file_browser"):
            if getattr(self.game.file_browser, "dragging_asset", False):
                self.game.file_browser.drop_dragged_asset()

        return True

    # =========================
    # HIERARCHY
    # =========================

    def handle_hierarchy_click(self, event):
        if event.type != pygame.MOUSEBUTTONDOWN:
            return False

        if not hasattr(self.game, "scene_hierarchy"):
            return False

        rect = self.get_panel_content_rect(
            "hierarchy",
            pygame.Rect(830, 70, 260, 170)
        )

        x, y = event.pos

        if not rect.collidepoint(x, y):
            return False

        search_rect = pygame.Rect(rect.x + 10, rect.y + 8, rect.width - 20, 20)
        tag_rect = pygame.Rect(rect.x + 10, rect.y + 31, 72, 18)
        layer_rect = pygame.Rect(rect.x + 88, rect.y + 31, 88, 18)
        reset_rect = pygame.Rect(rect.right - 54, rect.y + 31, 44, 18)

        if event.button == 1:
            if search_rect.collidepoint(x, y):
                self.game.scene_hierarchy.begin_search()
                return True

            if tag_rect.collidepoint(x, y):
                self.game.scene_hierarchy.cycle_tag_filter()
                return True

            if layer_rect.collidepoint(x, y):
                self.game.scene_hierarchy.cycle_layer_filter()
                return True

            if reset_rect.collidepoint(x, y):
                self.game.scene_hierarchy.reset_filters()
                return True

        if event.button == 4:
            self.game.scene_hierarchy.scroll_up()
            return True

        if event.button == 5:
            self.game.scene_hierarchy.scroll_down()
            return True

        if event.button == 1:
            row_y = rect.y + 56
            row_height = 20
            index = (y - row_y) // row_height

            shift = self.shift_pressed()

            if 0 <= index < self.game.scene_hierarchy.max_visible:
                self.game.scene_hierarchy.select_by_index(index, shift)
                return True

        return True

    # =========================
    # SETTINGS PANEL
    # =========================

    def handle_settings_panel_click(self, event):
        if event.type != pygame.MOUSEBUTTONDOWN or event.button != 1:
            return False

        if not getattr(self.game, "active_settings_panel", None):
            return False

        screen_w, _ = self.game.screen.get_size()
        rect = pygame.Rect(screen_w - 360, 80, 340, 360)

        if not rect.collidepoint(event.pos):
            return False

        x, y = event.pos
        close_rect = pygame.Rect(rect.right - 32, rect.y + 10, 20, 20)

        if close_rect.collidepoint(x, y):
            self.game.close_settings_panel()
            return True

        if self.game.active_settings_panel == "Build":
            return self.handle_build_settings_click(rect, x, y)

        if self.game.active_settings_panel == "Viewport":
            return self.handle_viewport_settings_click(rect, x, y)

        if self.game.active_settings_panel == "TagsLayers":
            return self.handle_tags_layers_click(rect, x, y)

        if self.game.active_settings_panel in ["Input", "BuildProfiles", "Plugins"]:
            return self.handle_generic_settings_click(rect, x, y)

        return True

    def handle_generic_settings_click(self, rect, x, y):
        panel = self.game.active_settings_panel

        if panel == "Input":
            rows = self.game.get_input_settings_rows()
        elif panel == "BuildProfiles":
            rows = self.game.get_build_profile_rows()
        else:
            rows = self.game.get_plugin_rows()

        start_y = rect.y + 52

        for i, (key, value) in enumerate(rows[:12]):
            row = pygame.Rect(rect.x + 12, start_y + i * 26, rect.width - 24, 22)

            if not row.collidepoint(x, y):
                continue

            if panel == "BuildProfiles" and key == "active":
                self.game.cycle_build_profile()
            elif panel == "Input":
                current = ",".join(value) if isinstance(value, list) else value
                self.game.start_settings_edit(key, current)

            return True

        return True

    def handle_build_settings_click(self, rect, x, y):
        rows = self.game.get_build_settings_rows()
        start_y = rect.y + 52

        for i, (key, value) in enumerate(rows):
            row = pygame.Rect(rect.x + 12, start_y + i * 26, rect.width - 24, 22)

            if row.collidepoint(x, y):
                self.game.start_settings_edit(key, value)
                return True

        return True

    def handle_viewport_settings_click(self, rect, x, y):
        rows = self.game.get_viewport_settings_rows()
        start_y = rect.y + 52

        for i, (key, value) in enumerate(rows):
            row = pygame.Rect(rect.x + 12, start_y + i * 24, rect.width - 24, 20)

            if row.collidepoint(x, y):
                if isinstance(value, bool):
                    self.game.toggle_viewport_setting(key)
                else:
                    self.game.start_settings_edit(key, value)

                return True

        return True

    def handle_tags_layers_click(self, rect, x, y):
        add_tag = pygame.Rect(rect.x + 12, rect.y + 52, 92, 24)
        add_layer = pygame.Rect(rect.x + 112, rect.y + 52, 100, 24)
        cycle_tag = pygame.Rect(rect.x + 12, rect.y + 84, 120, 24)
        cycle_layer = pygame.Rect(rect.x + 140, rect.y + 84, 130, 24)

        if add_tag.collidepoint(x, y):
            self.game.add_tag_from_panel()
            return True

        if add_layer.collidepoint(x, y):
            self.game.add_layer_from_panel()
            return True

        if cycle_tag.collidepoint(x, y):
            self.game.cycle_selected_tag_from_panel()
            return True

        if cycle_layer.collidepoint(x, y):
            self.game.cycle_selected_layer_from_panel()
            return True

        return True

    # =========================
    # INSPECTOR
    # =========================

    def handle_inspector_section_click(self, event):
        if event.type != pygame.MOUSEBUTTONDOWN or event.button != 1:
            return False

        if not hasattr(self.game, "inspector_sections_open"):
            return False

        rect = self.get_panel_content_rect(
            "inspector",
            pygame.Rect(850, 250, 240, 390)
        )

        if not rect.collidepoint(event.pos):
            return False

        x, y = event.pos
        cursor_y = rect.y + 8

        for section_name in self.game.inspector_sections_open.keys():
            header = pygame.Rect(rect.x + 10, cursor_y, rect.width - 20, 22)

            if header.collidepoint(x, y):
                self.game.toggle_inspector_section(section_name)
                return True

            cursor_y += 24

            if self.game.inspector_sections_open.get(section_name, True):
                if section_name == "Entity":
                    cursor_y += 116
                elif section_name == "Transform":
                    cursor_y += 182
                elif section_name == "Movement":
                    cursor_y += 74
                elif section_name == "Render":
                    cursor_y += 74
                elif section_name == "RTS":
                    cursor_y += 48
                elif section_name == "Components":
                    cursor_y += 100
                elif section_name == "Scripts":
                    cursor_y += 60
                elif section_name == "Prefab":
                    cursor_y += 48
                elif section_name == "Debug":
                    cursor_y += 60

        return False

    def handle_inspector_click(self, event):
        if event.type != pygame.MOUSEBUTTONDOWN or event.button != 1:
            return False

        if not hasattr(self.game, "inspector_editor"):
            return False

        return self.game.inspector_editor.handle_click(event.pos)

    # =========================
    # MINIMAP
    # =========================

    def handle_minimap_click(self, event):
        if event.type != pygame.MOUSEBUTTONDOWN:
            return False

        if not hasattr(self.game, "grid"):
            return False

        content_rect = self.get_panel_content_rect(
            "minimap",
            pygame.Rect(830, 500, 260, 130)
        )

        map_rect = pygame.Rect(
            content_rect.x + 10,
            content_rect.y + 8,
            max(10, content_rect.width - 20),
            max(10, content_rect.height - 18)
        )

        x, y = event.pos

        if not map_rect.collidepoint(x, y):
            return False

        local_x = (x - map_rect.x) / map_rect.width
        local_y = (y - map_rect.y) / map_rect.height

        world_x = local_x * self.game.grid.width * self.game.grid.tile_size
        world_y = local_y * self.game.grid.height * self.game.grid.tile_size

        if event.button == 1:
            self.game.center_camera_on_world(world_x, world_y)
            return True

        if event.button == 3:
            grid_x = int(world_x // self.game.grid.tile_size)
            grid_y = int(world_y // self.game.grid.tile_size)

            if hasattr(self.game, "command_system"):
                self.game.command_system.move_units((grid_x, grid_y))

            return True

        return False

    # =========================
    # WORLD INPUT
    # =========================

    def handle_mouse_down(self, event):
        if event.button == 2:
            self.middle_pan = True
            self.middle_pan_last = event.pos
            return

        if event.button == 1:
            if hasattr(self.game, "is_mouse_over_ui"):
                self.mouse_down_over_ui = self.game.is_mouse_over_ui(event.pos)
            else:
                self.mouse_down_over_ui = False

            if self.mouse_down_over_ui:
                return

            if self.try_start_gizmo_drag(event.pos):
                return

            if self.game.active_tool == "Entity":
                self.game.place_entity_or_prefab_at_screen(event.pos)
                return

            if self.game.active_tool == "Tile":
                self.game.paint_tile_at_screen(event.pos)
                return

            if self.game.active_tool == "Obstacle":
                self.game.paint_obstacle_at_screen(event.pos, 1)
                return

            if self.game.active_tool == "Erase":
                self.game.paint_obstacle_at_screen(event.pos, 0)
                return

            self.dragging = True
            self.start_pos = event.pos
            self.current_pos = event.pos

        elif event.button == 3:
            if hasattr(self.game, "is_mouse_over_ui"):
                if not self.game.is_mouse_over_ui(event.pos):
                    self.right_click(event.pos)
            else:
                self.right_click(event.pos)

    def handle_mouse_up(self, event):
        if event.button == 2:
            self.middle_pan = False
            return

        if event.button != 1:
            return

        if self.gizmo_dragging:
            self.gizmo_dragging = False
            self.game.history.take_snapshot(f"Gizmo {getattr(self.game.scene_view_tools, 'gizmo_mode', 'Move')}")
            return

        if self.mouse_down_over_ui:
            self.dragging = False
            self.mouse_down_over_ui = False
            return

        if not self.dragging:
            return

        self.dragging = False
        self.current_pos = event.pos

        if self.game.active_tool not in ["Select", "Move"]:
            return

        x1, y1 = self.start_pos
        x2, y2 = event.pos

        distance_x = abs(x2 - x1)
        distance_y = abs(y2 - y1)

        shift = self.shift_pressed()

        if distance_x < self.drag_threshold and distance_y < self.drag_threshold:
            self.game.select_at_screen(x2, y2, shift)
        else:
            contains = self.command_pressed()
            self.game.select_in_box(x1, y1, x2, y2, shift, contains)

    def handle_mouse_motion(self, event):
        if self.gizmo_dragging:
            dx = event.pos[0] - self.gizmo_last_pos[0]
            dy = event.pos[1] - self.gizmo_last_pos[1]
            self.gizmo_last_pos = event.pos

            if hasattr(self.game, "scene_view_tools"):
                self.game.scene_view_tools.apply_screen_drag(dx, dy, self.game.scene_view_tools.gizmo_mode)

            return

        if self.middle_pan:
            mx, my = event.pos
            last_x, last_y = self.middle_pan_last

            dx = mx - last_x
            dy = my - last_y

            zoom = max(0.2, getattr(self.game.camera, "zoom", 1.0))

            self.game.camera.x -= dx / zoom
            self.game.camera.y -= dy / zoom

            self.middle_pan_last = event.pos

            if hasattr(self.game.camera, "clamp"):
                try:
                    self.game.camera.clamp()
                except Exception:
                    pass

            return

        if self.dragging:
            self.current_pos = event.pos

        if pygame.mouse.get_pressed()[0]:
            if hasattr(self.game, "is_mouse_over_ui"):
                if self.game.is_mouse_over_ui(event.pos):
                    return

            if self.game.active_tool == "Tile":
                self.game.paint_tile_at_screen(event.pos)

            elif self.game.active_tool == "Obstacle":
                self.game.paint_obstacle_at_screen(event.pos, 1)

            elif self.game.active_tool == "Erase":
                self.game.paint_obstacle_at_screen(event.pos, 0)

    def try_start_gizmo_drag(self, pos):
        if not getattr(self.game, "selected_units", []):
            return False

        if not hasattr(self.game, "scene_view_tools"):
            return False

        if self.game.active_tool not in ["Move", "Rotate", "Scale"]:
            return False

        for entity in self.game.selected_units:
            try:
                rect = self.game.get_unit_screen_rect(entity).inflate(24, 24)
            except Exception:
                continue

            if rect.collidepoint(pos):
                self.gizmo_dragging = True
                self.gizmo_last_pos = pos
                return True

        return False

    def right_click(self, pos):
        if self.game.active_tool not in ["Select", "Move"]:
            return

        if not hasattr(self.game, "screen_to_grid"):
            return

        grid_x, grid_y = self.game.screen_to_grid(pos)

        if hasattr(self.game, "command_system"):
            if hasattr(self.game.command_system, "command_right_click"):
                self.game.command_system.command_right_click(grid_x, grid_y)
            else:
                self.game.command_system.move_units((grid_x, grid_y))

    # =========================
    # DROP FILE
    # =========================

    def handle_drop_file(self, event):
        file_path = event.file

        self.safe_log(f"Archivo arrastrado al editor: {file_path}", "EDITOR")

        if hasattr(self.game, "import_external_file"):
            self.game.import_external_file(file_path)
            return

        if hasattr(self.game, "file_browser"):
            if hasattr(self.game.file_browser, "refresh"):
                self.game.file_browser.refresh()

    # =========================
    # HELPERS
    # =========================

    def safe_get(self, obj, name):
        if hasattr(obj, name):
            return getattr(obj, name)
        return None

    def safe_log(self, message, level="INFO"):
        if hasattr(self.game, "console"):
            try:
                self.game.console.log(message, level)
                return
            except Exception:
                pass

        if hasattr(self.game, "logger"):
            try:
                if level in ["WARNING", "WARN"]:
                    self.game.logger.warning(message)
                elif level == "ERROR":
                    self.game.logger.error(message)
                elif level == "DEBUG":
                    self.game.logger.debug(message)
                else:
                    self.game.logger.info(message)
                return
            except Exception:
                pass

        print(f"[{level}] {message}")
