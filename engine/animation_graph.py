class AnimationClip:
    def __init__(self, name="Clip", duration=1.0, frames=None, loop=True):
        self.name = name
        self.duration = max(0.01, float(duration))
        self.frames = frames or [
            {"time": 0.0, "sprite_name": None, "tint": (255, 255, 255), "scale_x": 1.0, "scale_y": 1.0},
        ]
        self.loop = loop

    def sample(self, time_value):
        if self.loop:
            time_value = time_value % self.duration
        else:
            time_value = min(time_value, self.duration)

        current = self.frames[0]

        for frame in self.frames:
            if frame.get("time", 0.0) <= time_value:
                current = frame
            else:
                break

        return current

    def serialize(self):
        return {
            "name": self.name,
            "duration": self.duration,
            "frames": self.frames,
            "loop": self.loop,
        }

    @classmethod
    def from_data(cls, data):
        return cls(
            data.get("name", "Clip"),
            data.get("duration", 1.0),
            data.get("frames", []),
            data.get("loop", True),
        )


class AnimatorState:
    def __init__(self, name, clip_name):
        self.name = name
        self.clip_name = clip_name
        self.transitions = []

    def add_transition(self, target, parameter=None, equals=True):
        self.transitions.append({
            "target": target,
            "parameter": parameter,
            "equals": equals,
        })

    def serialize(self):
        return {
            "name": self.name,
            "clip_name": self.clip_name,
            "transitions": self.transitions,
        }

    @classmethod
    def from_data(cls, data):
        state = cls(data.get("name", "State"), data.get("clip_name", "Idle"))
        state.transitions = data.get("transitions", [])
        return state


class AnimatorControllerAsset:
    def __init__(self, name="Default"):
        self.name = name
        self.default_state = "Idle"
        self.clips = {
            "Idle": AnimationClip("Idle", 1.0, [
                {"time": 0.0, "tint": (255, 255, 255), "scale_x": 1.0, "scale_y": 1.0},
                {"time": 0.5, "tint": (220, 235, 255), "scale_x": 1.03, "scale_y": 1.03},
            ]),
            "Move": AnimationClip("Move", 0.45, [
                {"time": 0.0, "tint": (200, 245, 255), "scale_x": 1.0, "scale_y": 1.0},
                {"time": 0.22, "tint": (150, 230, 255), "scale_x": 1.08, "scale_y": 0.96},
            ]),
        }
        self.states = {
            "Idle": AnimatorState("Idle", "Idle"),
            "Move": AnimatorState("Move", "Move"),
        }
        self.states["Idle"].add_transition("Move", "moving", True)
        self.states["Move"].add_transition("Idle", "moving", False)

    def evaluate_transition(self, state_name, parameters):
        state = self.states.get(state_name)

        if not state:
            return self.default_state

        for transition in state.transitions:
            parameter = transition.get("parameter")

            if parameter is None:
                return transition.get("target", state_name)

            if parameters.get(parameter) == transition.get("equals", True):
                return transition.get("target", state_name)

        return state_name

    def clip_for_state(self, state_name):
        state = self.states.get(state_name) or self.states.get(self.default_state)

        if not state:
            return None

        return self.clips.get(state.clip_name)

    def serialize(self):
        return {
            "name": self.name,
            "default_state": self.default_state,
            "clips": {name: clip.serialize() for name, clip in self.clips.items()},
            "states": {name: state.serialize() for name, state in self.states.items()},
        }

    @classmethod
    def from_data(cls, data):
        controller = cls(data.get("name", "Default"))
        controller.default_state = data.get("default_state", "Idle")
        controller.clips = {
            name: AnimationClip.from_data(clip_data)
            for name, clip_data in data.get("clips", {}).items()
        }
        controller.states = {
            name: AnimatorState.from_data(state_data)
            for name, state_data in data.get("states", {}).items()
        }
        return controller


class AnimationGraphLibrary:
    def __init__(self):
        self.controllers = {"Default": AnimatorControllerAsset("Default")}

    def get(self, name):
        return self.controllers.get(name) or self.controllers["Default"]

    def add(self, controller):
        self.controllers[controller.name] = controller

    def names(self):
        return sorted(self.controllers.keys())

    def serialize(self):
        return {
            "controllers": {
                name: controller.serialize()
                for name, controller in self.controllers.items()
            }
        }

    def deserialize(self, data):
        controllers = data.get("controllers", {})

        if controllers:
            self.controllers = {
                name: AnimatorControllerAsset.from_data(controller_data)
                for name, controller_data in controllers.items()
            }
