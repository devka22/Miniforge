import pygame


class Button:
    def __init__(self, x, y, w, h, text, callback, category="General"):
        self.rect = pygame.Rect(x, y, w, h)
        self.text = text
        self.callback = callback
        self.category = category

        self.hovered = False
        self.font = None

    def draw(self, screen):
        if self.font is None:
            self.font = pygame.font.SysFont(None, 19)

        mx, my = pygame.mouse.get_pos()
        self.hovered = self.rect.collidepoint(mx, my)

        base = (246, 247, 250)
        hover = (225, 232, 255)
        border = (190, 195, 205)
        text_color = (35, 35, 40)

        color = hover if self.hovered else base

        pygame.draw.rect(screen, color, self.rect, border_radius=7)
        pygame.draw.rect(screen, border, self.rect, 1, border_radius=7)

        img = self.font.render(self.text, True, text_color)
        screen.blit(
            img,
            (
                self.rect.x + 10,
                self.rect.y + (self.rect.height - img.get_height()) // 2
            )
        )

    def handle_event(self, event):
        if event.type == pygame.MOUSEBUTTONDOWN and event.button == 1:
            if self.rect.collidepoint(event.pos):
                self.callback()
                return True

        return False