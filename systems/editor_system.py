import pygame


class EditorSystem:
    def __init__(self, game):
        self.game = game
        self.selected = None
        self.update_when_paused = True

        self.key_locks = {
            "spawn": False,
            "save": False,
            "load": False,
            "console": False,
            "patrol": False,
            "color": False,
            "duplicate": False,
            "delete": False,
            "undo": False,
            "redo": False,
        }

    def update(self, dt):
        if hasattr(self.game, "ui_captures_keyboard"):
            if self.game.ui_captures_keyboard():
                self.update_selected()
                return

        keys = pygame.key.get_pressed()

        self.update_selected()
        self.handle_global_hotkeys(keys)
        self.handle_editor_hotkeys(keys)
        self.handle_inspector_movement(keys, dt)

    def update_selected(self):
        if self.game.selected_units:
            self.selected = self.game.selected_units[0]
        else:
            self.selected = None

    def handle_global_hotkeys(self, keys):
        if keys[pygame.K_F1]:
            if not self.key_locks["console"]:
                self.game.console.toggle()
                self.key_locks["console"] = True
        else:
            self.key_locks["console"] = False

    def handle_editor_hotkeys(self, keys):
        if self.game.mode != "EDITOR":
            return

        command = keys[pygame.K_LMETA] or keys[pygame.K_RMETA] or keys[pygame.K_LCTRL] or keys[pygame.K_RCTRL]

        if keys[pygame.K_1]:
            if not self.key_locks["spawn"]:
                self.game.spawn_unit()
                self.key_locks["spawn"] = True
        else:
            self.key_locks["spawn"] = False

        if keys[pygame.K_F5]:
            if not self.key_locks["save"]:
                self.game.save_scene()
                self.key_locks["save"] = True
        else:
            self.key_locks["save"] = False

        if keys[pygame.K_F9]:
            if not self.key_locks["load"]:
                self.game.load_scene()
                self.key_locks["load"] = True
        else:
            self.key_locks["load"] = False

        if keys[pygame.K_2]:
            if not self.key_locks["patrol"]:
                self.game.add_patrol_script()
                self.key_locks["patrol"] = True
        else:
            self.key_locks["patrol"] = False

        if keys[pygame.K_3]:
            if not self.key_locks["color"]:
                self.game.add_color_script()
                self.key_locks["color"] = True
        else:
            self.key_locks["color"] = False

        if keys[pygame.K_d]:
            if not self.key_locks["duplicate"]:
                self.game.duplicate_selected()
                self.key_locks["duplicate"] = True
        else:
            self.key_locks["duplicate"] = False

        if keys[pygame.K_BACKSPACE] or keys[pygame.K_DELETE]:
            if not self.key_locks["delete"]:
                self.game.delete_selected()
                self.key_locks["delete"] = True
        else:
            self.key_locks["delete"] = False

        if command and keys[pygame.K_z]:
            if not self.key_locks["undo"]:
                self.game.undo()
                self.key_locks["undo"] = True
        else:
            self.key_locks["undo"] = False

        if command and keys[pygame.K_y]:
            if not self.key_locks["redo"]:
                self.game.redo()
                self.key_locks["redo"] = True
        else:
            self.key_locks["redo"] = False

    def handle_inspector_movement(self, keys, dt):
        if self.game.mode != "EDITOR":
            return

        if not self.selected:
            return

        speed = 5 * dt

        changed = False

        if keys[pygame.K_UP]:
            self.selected.y -= speed
            changed = True

        if keys[pygame.K_DOWN]:
            self.selected.y += speed
            changed = True

        if keys[pygame.K_LEFT]:
            self.selected.x -= speed
            changed = True

        if keys[pygame.K_RIGHT]:
            self.selected.x += speed
            changed = True

        if changed:
            self.selected.x = round(self.selected.x, 3)
            self.selected.y = round(self.selected.y, 3)
