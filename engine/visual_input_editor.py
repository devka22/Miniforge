class VisualInputEditor:
    """
    Editor visual de input.
    - lista acciones
    - captura tecla/mouse
    - añade/remueve bindings
    - crea acciones nuevas hardcodeadas desde UI
    """

    PRESET_ACTIONS = [
        "move_up",
        "move_down",
        "move_left",
        "move_right",
        "jump",
        "attack",
        "interact",
        "pause",
        "dash",
        "inventory",
        "map",
        "skill_1",
        "skill_2",
    ]

    def __init__(self, game):
        self.game = game
        self.visible = False
        self.selected_action = "jump"
        self.capture_mode = False
        self.scroll = 0

    def toggle(self):
        self.visible = not self.visible
        self.capture_mode = False

    def actions(self):
        existing = list(self.game.input_map.bindings.keys())

        for action in self.PRESET_ACTIONS:
            if action not in existing:
                existing.append(action)

        return existing

    def select(self, action):
        self.selected_action = action

        if action not in self.game.input_map.bindings:
            self.game.input_map.set_binding(action, [])

    def start_capture(self):
        self.capture_mode = True

    def add_binding(self, key_name):
        self.select(self.selected_action)
        bindings = list(self.game.input_map.bindings.get(self.selected_action, []))

        if key_name not in bindings:
            bindings.append(key_name)

        self.game.input_map.set_binding(self.selected_action, bindings)
        self.capture_mode = False
        self.game.console.log(f"Input {self.selected_action}: {bindings}", "ENGINE")

    def remove_last_binding(self):
        bindings = list(self.game.input_map.bindings.get(self.selected_action, []))

        if bindings:
            bindings.pop()
            self.game.input_map.set_binding(self.selected_action, bindings)

    def create_next_action(self):
        index = 1

        while f"action_{index}" in self.game.input_map.bindings:
            index += 1

        action = f"action_{index}"
        self.game.input_map.set_binding(action, [])
        self.select(action)
        return action
