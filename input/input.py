from engine.animation import AnimationController, ColorAnimation


class Unit:
    def __init__(self, x, y, game):
        self.game = game

        self.x = float(x)
        self.y = float(y)

        self.selected = False
        self.path = []

        self.speed = 3.5
        self.radius = 0.45

        self.state = "IDLE"

        self.components = []
        self.scripts = []

        # Sprite opcional del ResourceManager
        self.sprite_name = None

        self.debug_color_flip = False

        self.animator = AnimationController()

        self.animator.add(
            "IDLE",
            ColorAnimation([(0, 120, 255), (0, 140, 255)], speed=0.6),
        )

        self.animator.add(
            "MOVING",
            ColorAnimation([(0, 200, 255), (0, 255, 200)], speed=0.15),
        )

        self.animator.add(
            "SELECTED",
            ColorAnimation([(0, 255, 0), (120, 255, 120)], speed=0.2),
        )

    def add_component(self, component):
        self.components.append(component)
        component.start(self)

    def add_script(self, script):
        self.scripts.append(script)
        script.start(self)

    def set_selected(self, value):
        if self.selected == value:
            return

        self.selected = value

        if self.selected:
            for script in self.scripts:
                script.on_selected(self)
        else:
            for script in self.scripts:
                script.on_deselected(self)

    def update(self, dt=0.016):
        for component in self.components:
            if component.enabled:
                try:
                    component.update(self, dt)
                except Exception as error:
                    self.game.console.log(f"Component error: {error}")

        # Los scripts personalizados corren solo en modo PLAY
        if self.game.mode == "PLAY":
            for script in self.scripts:
                if script.enabled:
                    try:
                        if not script.started:
                            script.start(self)
                            script.started = True

                        script.update(self, dt)
                    except Exception as error:
                        self.game.console.log(f"Script error: {error}")

        self.update_movement(dt)
        self.apply_separation()
        self.update_animation(dt)

    def update_movement(self, dt):
        if not self.path:
            self.state = "IDLE"
            return

        self.state = "MOVING"

        tx, ty = self.path[0]

        dx = tx - self.x
        dy = ty - self.y

        dist = (dx * dx + dy * dy) ** 0.5

        if dist < 0.08:
            self.path.pop(0)
            return

        if dist > 0:
            step = self.speed * dt
            self.x += (dx / dist) * step
            self.y += (dy / dist) * step

    def apply_separation(self):
        for other in self.game.units:
            if other is self:
                continue

            dx = self.x - other.x
            dy = self.y - other.y

            dist = (dx * dx + dy * dy) ** 0.5
            min_dist = self.radius + other.radius

            if 0 < dist < min_dist:
                push = (min_dist - dist) * 0.03
                self.x += (dx / dist) * push
                self.y += (dy / dist) * push

    def update_animation(self, dt):
        if self.selected:
            self.animator.play("SELECTED")
        elif self.state == "MOVING":
            self.animator.play("MOVING")
        else:
            self.animator.play("IDLE")

        self.animator.update(dt)

    def get_color(self):
        if self.debug_color_flip:
            return (255, 200, 0)

        return self.animator.get_color()

    def serialize(self):
        return {
            "type": "Unit",
            "x": self.x,
            "y": self.y,
            "speed": self.speed,
            "sprite_name": self.sprite_name,
            "scripts": [script.serialize() for script in self.scripts]
        }