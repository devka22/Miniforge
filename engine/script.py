class Script:
    script_name = "BaseScript"

    def __init__(self):
        self.enabled = True
        self.started = False

    def start(self, entity):
        pass

    def update(self, entity, dt):
        pass

    def on_selected(self, entity):
        pass

    def on_deselected(self, entity):
        pass

    def serialize(self):
        return {
            "script": self.script_name,
            "enabled": self.enabled
        }

    def deserialize(self, data):
        self.enabled = data.get("enabled", True)


def invoke_script_method(script, method_name, entity=None, dt=None):
    """
    Ejecuta scripts antiguos y nuevos sin romper compatibilidad.
    Soporta firmas:
    - start(self, entity)
    - start(self)
    - update(self, entity, dt)
    - update(self, dt)
    - update(self)
    """
    method = getattr(script, method_name, None)

    if not method:
        return None

    attempts = []

    if entity is not None and dt is not None:
        attempts.append((entity, dt))

    if dt is not None:
        attempts.append((dt,))

    if entity is not None:
        attempts.append((entity,))

    attempts.append(())

    last_error = None

    for args in attempts:
        try:
            return method(*args)
        except TypeError as error:
            last_error = error
            continue

    if last_error:
        raise last_error

    return None
