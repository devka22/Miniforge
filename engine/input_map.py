import pygame
import json
import os

class InputMap:
    DEFAULT_BINDINGS = {
        "move_up": ["w", "up"],
        "move_down": ["s", "down"],
        "move_left": ["a", "left"],
        "move_right": ["d", "right"],
        "jump": ["space"],
        "run": ["left_shift"],
        "dash": ["left_shift"],
        "attack": ["mouse1"],
        "secondary_attack": ["mouse2"],
        "interact": ["e"],
        "inventory": ["i"],
        "map": ["m"],
        "ability_1": ["1"],
        "ability_2": ["2"],
        "ability_3": ["3"],
        "ability_4": ["4"],
        "pause": ["escape"],
        "save": ["s"],
        "duplicate": ["d"],
    }

    def __init__(self, path=None):
        self.path = path or os.path.join("settings", "input_map.json")
        self.bindings = {
            action: list(keys)
            for action, keys in self.DEFAULT_BINDINGS.items()
        }
        self.load()

    def is_action(self, action):
        return self.get_action(action)

    def get_action(self, action):
        keys = pygame.key.get_pressed()
        mouse = pygame.mouse.get_pressed()

        for key_name in self.bindings.get(action, []):
            if key_name == "mouse1" and mouse[0]:
                return True

            if key_name == "mouse2" and mouse[2]:
                return True

            key = self.key_code(key_name)

            if key is not None and keys[key]:
                return True

        return False

    def set_binding(self, action, key_names):
        self.bindings[action] = list(key_names)
        self.save()

    def key_code(self, key_name):
        if isinstance(key_name, int):
            return key_name

        name = str(key_name).lower()

        aliases = {
            "space": pygame.K_SPACE,
            "escape": pygame.K_ESCAPE,
            "esc": pygame.K_ESCAPE,
            "up": pygame.K_UP,
            "down": pygame.K_DOWN,
            "left": pygame.K_LEFT,
            "right": pygame.K_RIGHT,
            "enter": pygame.K_RETURN,
            "return": pygame.K_RETURN,
        }

        if name in aliases:
            return aliases[name]

        if len(name) == 1:
            return getattr(pygame, f"K_{name}", None)

        return getattr(pygame, f"K_{name}", None)

    def load(self):
        if not os.path.exists(self.path):
            self.save()
            return

        try:
            with open(self.path, "r", encoding="utf-8") as file:
                data = json.load(file)

            self.bindings.update(data.get("bindings", {}))
        except Exception:
            self.save()

    def save(self):
        os.makedirs(os.path.dirname(self.path), exist_ok=True)

        with open(self.path, "w", encoding="utf-8") as file:
            json.dump({"bindings": self.bindings}, file, indent=4)
