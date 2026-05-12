import pygame


class Toolbar:
    """
    Toolbar compacta, limpia y ordenada.
    """

    def __init__(self, game):
        self.game = game
        self.height = 30
        self.y = 34
        self.font = pygame.font.SysFont(None, 18)
        self.buttons = []

    def get_tools(self):
        return [
            ("Select", "Select"),
            ("Move", "Move"),
            ("Entity", "Entity"),
            ("Tile", "Tile"),
            ("Obstacle", "Obstacle"),
            ("Erase", "Erase"),
        ]

    def is_mouse_over(self, pos):
        x, y = pos
        return self.y <= y <= self.y + self.height

    def handle_event(self, event):
        if event.type != pygame.MOUSEBUTTONDOWN or event.button != 1:
            return False

        for rect, kind, value in self.buttons:
            if rect.collidepoint(event.pos):
                if kind == "tool":
                    self.game.active_tool = value
                    self.game.console.log(f"Tool: {value}", "ENGINE")
                else:
                    value()

                return True

        return False

    def draw(self, screen):
        width = screen.get_width()
        bar = pygame.Rect(0, self.y, width, self.height)

        pygame.draw.rect(screen, (238, 240, 245), bar)
        pygame.draw.line(
            screen,
            (205, 208, 218),
            (0, self.y + self.height),
            (width, self.y + self.height)
        )

        self.buttons.clear()

        x = 170
        tool_width = 70
        tool_gap = 74

        for label, tool in self.get_tools():
            active = self.game.active_tool == tool
            rect = pygame.Rect(x, self.y + 4, tool_width, 22)

            if active:
                pygame.draw.rect(screen, (0, 122, 255), rect, border_radius=7)
                color = (255, 255, 255)

            else:
                if rect.collidepoint(pygame.mouse.get_pos()):
                    pygame.draw.rect(screen, (225, 232, 248), rect, border_radius=7)
                else:
                    pygame.draw.rect(screen, (250, 250, 252), rect, border_radius=7)

                pygame.draw.rect(screen, (205, 208, 218), rect, 1, border_radius=7)
                color = (45, 48, 56)

            img = self.font.render(label, True, color)

            screen.blit(
                img,
                (
                    rect.x + (rect.width - img.get_width()) // 2,
                    rect.y + 5
                )
            )

            self.buttons.append((rect, "tool", tool))
            x += tool_gap

        actions = [
            ("Play", self.game.play),
            ("Stop", self.game.stop),
            ("Pause", self.game.pause_play_mode),
            ("Game", self.game.toggle_view_mode),
            ("Check", self.game.validate_project),
            ("Export", self.game.export_build),
        ]

        x += 12

        for label, callback in actions:
            rect = pygame.Rect(x, self.y + 4, 58, 22)
            active = (
                (label == "Play" and self.game.mode == "PLAY")
                or (
                    label == "Pause"
                    and getattr(getattr(self.game, "play_mode_manager", None), "paused", False)
                )
            )

            if active:
                pygame.draw.rect(screen, (35, 150, 90), rect, border_radius=7)
                color = (255, 255, 255)
            else:
                if rect.collidepoint(pygame.mouse.get_pos()):
                    pygame.draw.rect(screen, (225, 232, 248), rect, border_radius=7)
                else:
                    pygame.draw.rect(screen, (250, 250, 252), rect, border_radius=7)

                pygame.draw.rect(screen, (205, 208, 218), rect, 1, border_radius=7)
                color = (45, 48, 56)

            img = self.font.render(label, True, color)
            screen.blit(
                img,
                (
                    rect.x + (rect.width - img.get_width()) // 2,
                    rect.y + 5
                )
            )

            self.buttons.append((rect, "action", callback))
            x += 62

        mode_text = f"{self.game.mode} | {self.game.view_mode.mode}"
        img = self.font.render(mode_text, True, (80, 84, 96))

        if width - img.get_width() - 16 > x + 8:
            screen.blit(img, (width - img.get_width() - 16, self.y + 8))
