class EventBus:
    def __init__(self):
        self.listeners = {}

    def subscribe(self, event, callback):
        if event not in self.listeners:
            self.listeners[event] = []
        self.listeners[event].append(callback)

    def emit(self, event, data=None):
        if event in self.listeners:
            for callback in self.listeners[event]:
                callback(data)