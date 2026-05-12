import time


class ColorAnimation:
    def __init__(self, colors, speed=0.25):
        self.colors = colors or [(0, 120, 255)]
        self.speed = max(0.01, float(speed))
        self.index = 0
        self.timer = 0.0
        self.current_color = self.colors[0]

    def update(self, dt):
        self.timer += dt

        if self.timer >= self.speed:
            self.timer = 0.0
            self.index = (self.index + 1) % len(self.colors)
            self.current_color = self.colors[self.index]

    def get_color(self):
        return self.current_color


class AnimationController:
    def __init__(self):
        self.animations = {}
        self.current_name = None
        self.current = None

    def add(self, name, animation):
        self.animations[name] = animation

        if self.current is None:
            self.play(name)

    def play(self, name):
        if name == self.current_name:
            return

        if name not in self.animations:
            return

        self.current_name = name
        self.current = self.animations[name]

    def update(self, dt):
        if self.current:
            self.current.update(dt)

    def get_color(self):
        if self.current:
            return self.current.get_color()

        return (0, 120, 255)