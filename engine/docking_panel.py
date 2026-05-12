import pygame


class DockingPanel:
    """
    Panel movible y escalable.
    Sirve para Inspector, Console, Hierarchy, Minimap, Content Preview, etc.
    """

    def __init__(self, panel_id, title, x, y, width, height):
        self.panel_id = panel_id
        self.title = title

        self.rect = pygame.Rect(x, y, width, height)

        self.visible = True
        self.collapsed = False

        self.dragging = False
        self.resizing = False

        self.drag_offset = (0, 0)

        self.min_width = 180
        self.min_height = 80

        self.title_height = 28
        self.resize_size = 14

    def title_rect(self):
        return pygame.Rect(
            self.rect.x,
            self.rect.y,
            self.rect.width,
            self.title_height
        )

    def content_rect(self):
        if self.collapsed:
            return pygame.Rect(self.rect.x, self.rect.y + self.title_height, self.rect.width, 0)

        return pygame.Rect(
            self.rect.x,
            self.rect.y + self.title_height,
            self.rect.width,
            self.rect.height - self.title_height
        )

    def resize_rect(self):
        return pygame.Rect(
            self.rect.right - self.resize_size,
            self.rect.bottom - self.resize_size,
            self.resize_size,
            self.resize_size
        )

    def close_rect(self):
        return pygame.Rect(
            self.rect.right - 24,
            self.rect.y + 6,
            14,
            14
        )

    def collapse_rect(self):
        return pygame.Rect(
            self.rect.right - 44,
            self.rect.y + 6,
            14,
            14
        )

    def is_mouse_over(self, pos):
        return self.visible and self.rect.collidepoint(pos)

    def handle_event(self, event):
        if not self.visible:
            return False

        if event.type == pygame.MOUSEBUTTONDOWN and event.button == 1:
            if self.close_rect().collidepoint(event.pos):
                self.visible = False
                return True

            if self.collapse_rect().collidepoint(event.pos):
                self.collapsed = not self.collapsed
                return True

            if self.resize_rect().collidepoint(event.pos):
                self.resizing = True
                return True

            if self.title_rect().collidepoint(event.pos):
                self.dragging = True
                self.drag_offset = (
                    event.pos[0] - self.rect.x,
                    event.pos[1] - self.rect.y
                )
                return True

        if event.type == pygame.MOUSEBUTTONUP and event.button == 1:
            if self.dragging or self.resizing:
                self.dragging = False
                self.resizing = False
                return True

        if event.type == pygame.MOUSEMOTION:
            if self.dragging:
                self.rect.x = event.pos[0] - self.drag_offset[0]
                self.rect.y = event.pos[1] - self.drag_offset[1]
                return True

            if self.resizing:
                self.rect.width = max(
                    self.min_width,
                    event.pos[0] - self.rect.x
                )
                self.rect.height = max(
                    self.min_height,
                    event.pos[1] - self.rect.y
                )
                return True

        return False

    def draw_base(self, screen, font, small_font):
        if not self.visible:
            return

        pygame.draw.rect(
            screen,
            (250, 250, 252),
            self.rect,
            border_radius=10
        )

        pygame.draw.rect(
            screen,
            (205, 208, 218),
            self.rect,
            1,
            border_radius=10
        )

        title_rect = self.title_rect()

        pygame.draw.rect(
            screen,
            (235, 237, 242),
            title_rect,
            border_top_left_radius=10,
            border_top_right_radius=10
        )

        title_img = small_font.render(self.title, True, (35, 36, 42))
        screen.blit(title_img, (self.rect.x + 10, self.rect.y + 7))

        # Collapse button
        pygame.draw.rect(
            screen,
            (220, 224, 232),
            self.collapse_rect(),
            border_radius=4
        )

        collapse_text = "-" if not self.collapsed else "+"
        img = small_font.render(collapse_text, True, (60, 60, 65))
        screen.blit(img, (self.collapse_rect().x + 4, self.collapse_rect().y - 1))

        # Close button
        pygame.draw.rect(
            screen,
            (235, 120, 120),
            self.close_rect(),
            border_radius=4
        )

        # Resize handle
        if not self.collapsed:
            pygame.draw.line(
                screen,
                (160, 165, 175),
                (self.rect.right - 12, self.rect.bottom - 4),
                (self.rect.right - 4, self.rect.bottom - 12),
                2
            )