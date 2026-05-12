class RenderSystem:
    def __init__(self, renderer):
        self.renderer = renderer

    def draw(self):
        self.renderer.draw()