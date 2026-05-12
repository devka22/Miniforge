import pygame


class CameraSystem:
    def __init__(self, game):
        self.game = game
        self.speed = 520

    def update(self, dt):
        if hasattr(self.game, "ui_captures_keyboard"):
            if self.game.ui_captures_keyboard():
                return

        cam = self.game.camera
        keys = pygame.key.get_pressed()

        dx = 0
        dy = 0

        input_map = getattr(self.game, "input_map", None)

        if input_map and input_map.get_action("move_left"):
            dx -= self.speed * dt
        elif keys[pygame.K_a]:
            dx -= self.speed * dt

        if input_map and input_map.get_action("move_right"):
            dx += self.speed * dt
        elif keys[pygame.K_d]:
            dx += self.speed * dt

        if input_map and input_map.get_action("move_up"):
            dy -= self.speed * dt
        elif keys[pygame.K_w]:
            dy -= self.speed * dt

        if input_map and input_map.get_action("move_down"):
            dy += self.speed * dt
        elif keys[pygame.K_s]:
            dy += self.speed * dt

        mx, my = pygame.mouse.get_pos()
        w, h = self.game.screen.get_size()

        margin = 18

        if mx < margin:
            dx -= self.speed * dt
        if mx > w - margin:
            dx += self.speed * dt
        if my < margin:
            dy -= self.speed * dt
        if my > h - margin:
            dy += self.speed * dt

        cam.move(dx, dy)

        # Zoom
        if keys[pygame.K_q]:
            cam.zoom_by(-0.8 * dt)

        if keys[pygame.K_e]:
            cam.zoom_by(0.8 * dt)
