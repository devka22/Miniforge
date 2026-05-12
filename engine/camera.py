class Camera:
    def __init__(self):
        self.x = 0.0
        self.y = 0.0
        self.zoom = 1.0

        self.min_zoom = 0.5
        self.max_zoom = 2.0

        self.bounds = None
        self.viewport = {
            "x": 0,
            "y": 0,
            "width": 1,
            "height": 1,
        }

    def set_bounds(self, x, y, width, height):
        self.bounds = {
            "x": x,
            "y": y,
            "width": width,
            "height": height,
        }
        self.clamp()

    def set_viewport(self, rect):
        self.viewport = {
            "x": int(rect.x),
            "y": int(rect.y),
            "width": max(1, int(rect.width)),
            "height": max(1, int(rect.height)),
        }
        self.clamp()

    def move(self, dx, dy):
        self.x += dx
        self.y += dy
        self.clamp()

    def set_zoom(self, zoom):
        self.zoom = max(self.min_zoom, min(self.max_zoom, zoom))
        self.clamp()

    def zoom_by(self, amount):
        self.set_zoom(self.zoom + amount)

    def world_to_screen(self, wx, wy):
        sx = (wx - self.x) * self.zoom + self.viewport["x"]
        sy = (wy - self.y) * self.zoom + self.viewport["y"]
        return sx, sy

    def screen_to_world(self, sx, sy):
        wx = (sx - self.viewport["x"]) / self.zoom + self.x
        wy = (sy - self.viewport["y"]) / self.zoom + self.y
        return wx, wy

    def clamp(self):
        if not self.bounds:
            return

        self.x = max(self.bounds["x"], self.x)
        self.y = max(self.bounds["y"], self.y)

        view_width = self.viewport["width"] / max(self.zoom, 0.001)
        view_height = self.viewport["height"] / max(self.zoom, 0.001)

        max_x = max(self.bounds["x"], self.bounds["width"] - view_width)
        max_y = max(self.bounds["y"], self.bounds["height"] - view_height)

        self.x = min(self.x, max_x)
        self.y = min(self.y, max_y)
