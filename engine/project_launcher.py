import pygame

from engine.project_system import ProjectSystem


class ProjectLauncher:
    """
    Launcher básico tipo Unity Hub.
    Permite abrir proyecto reciente o crear DefaultProject.
    """

    def __init__(self):
        pygame.init()

        self.project_system = ProjectSystem()

        self.screen = pygame.display.set_mode((620, 420))
        pygame.display.set_caption("MiniForge Launcher")

        self.clock = pygame.time.Clock()
        self.font = pygame.font.SysFont(None, 28)
        self.small_font = pygame.font.SysFont(None, 20)

        self.running = True
        self.selected_project = None

        self.buttons = []

    def run(self):
        recent = self.project_system.get_recent_projects()

        if recent:
            self.selected_project = recent[0]
        else:
            self.selected_project = self.project_system.create_project("DefaultProject")

        while self.running:
            self.clock.tick(60)

            for event in pygame.event.get():
                if event.type == pygame.QUIT:
                    self.running = False
                    return self.selected_project

                if event.type == pygame.KEYDOWN:
                    if event.key == pygame.K_RETURN:
                        self.running = False
                        return self.selected_project

                    if event.key == pygame.K_n:
                        self.selected_project = self.project_system.create_project("NewProject")
                        self.running = False
                        return self.selected_project

                    if event.key == pygame.K_d:
                        self.selected_project = self.project_system.get_default_project()
                        self.running = False
                        return self.selected_project

                if event.type == pygame.MOUSEBUTTONDOWN and event.button == 1:
                    self.handle_click(event.pos)

            self.draw()

        return self.selected_project

    def handle_click(self, pos):
        for rect, action, value in self.buttons:
            if rect.collidepoint(pos):
                if action == "open":
                    self.selected_project = value
                    self.running = False

                elif action == "default":
                    self.selected_project = self.project_system.get_default_project()
                    self.running = False

                elif action == "new":
                    self.selected_project = self.project_system.create_project("NewProject")
                    self.running = False

    def draw(self):
        self.screen.fill((240, 242, 247))
        self.buttons.clear()

        title = self.font.render("MiniForge Launcher", True, (30, 32, 38))
        self.screen.blit(title, (30, 30))

        subtitle = self.small_font.render(
            "Enter: Open selected | N: New Project | D: Default Project",
            True,
            (90, 94, 105)
        )
        self.screen.blit(subtitle, (30, 65))

        new_rect = pygame.Rect(30, 105, 170, 36)
        default_rect = pygame.Rect(220, 105, 170, 36)

        self.draw_button(new_rect, "New Project")
        self.draw_button(default_rect, "Open Default")

        self.buttons.append((new_rect, "new", None))
        self.buttons.append((default_rect, "default", None))

        recent_title = self.small_font.render("Recent Projects", True, (70, 74, 84))
        self.screen.blit(recent_title, (30, 165))

        recent = self.project_system.get_recent_projects()

        y = 195

        if not recent:
            empty = self.small_font.render("No recent projects.", True, (120, 120, 130))
            self.screen.blit(empty, (30, y))
        else:
            for path in recent[:6]:
                rect = pygame.Rect(30, y, 560, 34)

                selected = path == self.selected_project

                color = (215, 228, 255) if selected else (250, 250, 252)

                pygame.draw.rect(self.screen, color, rect, border_radius=8)
                pygame.draw.rect(self.screen, (200, 204, 215), rect, 1, border_radius=8)

                name = self.small_font.render(path, True, (35, 36, 42))
                self.screen.blit(name, (rect.x + 10, rect.y + 9))

                self.buttons.append((rect, "open", path))

                y += 42

        pygame.display.flip()

    def draw_button(self, rect, text):
        pygame.draw.rect(self.screen, (250, 250, 252), rect, border_radius=8)
        pygame.draw.rect(self.screen, (200, 204, 215), rect, 1, border_radius=8)

        img = self.small_font.render(text, True, (35, 36, 42))
        self.screen.blit(
            img,
            (
                rect.x + (rect.width - img.get_width()) // 2,
                rect.y + 9
            )
        )