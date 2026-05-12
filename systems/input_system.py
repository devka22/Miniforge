class InputSystem:
    def __init__(self, input_handler):
        self.input = input_handler
        self.update_when_paused = True

    def update(self, dt):
        self.input.handle_events()
