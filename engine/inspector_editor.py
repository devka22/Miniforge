class InspectorEditor:
    """
    Inspector editable avanzado:
    - campos de entidad
    - campos de componentes
    - enabled de componentes
    """

    def __init__(self, game):
        self.game = game

        self.editing = False
        self.field = None
        self.buffer = ""

        self.field_rects = {}

    def begin_edit(self, field, current_value):
        self.editing = True
        self.field = field
        self.buffer = "" if current_value is None else str(current_value)
        self.game.console.log(f"Editando: {field}", "ENGINE")

    def cancel(self):
        self.editing = False
        self.field = None
        self.buffer = ""

    def handle_key(self, event):
        import pygame

        if not self.editing:
            return False

        if event.key == pygame.K_ESCAPE:
            self.cancel()
            return True

        if event.key == pygame.K_RETURN:
            self.apply()
            return True

        if event.key == pygame.K_BACKSPACE:
            self.buffer = self.buffer[:-1]
            return True

        if event.unicode and event.unicode.isprintable():
            self.buffer += event.unicode
            return True

        return False

    def parse_bool(self, value):
        return str(value).lower() in ["true", "1", "yes", "y", "si", "sí"]

    def apply(self):
        if not self.game.selected_units:
            self.cancel()
            return

        unit = self.game.selected_units[0]
        value = self.buffer

        try:
            # Formato especial:
            # component:RTSMovement:speed
            if self.field.startswith("component:"):
                _, component_type, attr = self.field.split(":", 2)

                component = unit.get_component(component_type)

                if not component:
                    self.game.console.log(f"No existe componente {component_type}", "WARNING")
                    self.cancel()
                    return

                old_value = getattr(component, attr, None)

                if isinstance(old_value, bool):
                    setattr(component, attr, self.parse_bool(value))

                elif isinstance(old_value, int):
                    setattr(component, attr, int(value))

                elif isinstance(old_value, float):
                    setattr(component, attr, float(value))

                else:
                    setattr(component, attr, value if value else None)

                unit.sync_from_components()
                unit.sync_to_components()

                self.game.history.take_snapshot(f"Edit {component_type}.{attr}")
                self.game.console.log(f"{component_type}.{attr} cambiado a {value}", "ENGINE")
                self.cancel()
                return

            # Campos normales de entidad
            if self.field == "name":
                unit.name = value if value else unit.name

            elif self.field == "enabled":
                unit.enabled = self.parse_bool(value)
                unit.active = unit.enabled

            elif self.field == "active":
                unit.enabled = self.parse_bool(value)
                unit.active = unit.enabled

            elif self.field == "visible":
                unit.visible = self.parse_bool(value)

            elif self.field == "locked":
                unit.locked = self.parse_bool(value)

            elif self.field == "x":
                unit.x = float(value)

            elif self.field == "y":
                unit.y = float(value)

            elif self.field == "rotation":
                unit.rotation = float(value)

            elif self.field == "scale_x":
                unit.scale_x = float(value)

            elif self.field == "scale_y":
                unit.scale_y = float(value)

            elif self.field == "width":
                unit.width = float(value)

            elif self.field == "height":
                unit.height = float(value)

            elif self.field == "local_x":
                unit.local_x = float(value)

            elif self.field == "local_y":
                unit.local_y = float(value)

            elif self.field == "speed":
                unit.speed = float(value)

            elif self.field == "radius":
                unit.radius = float(value)

            elif self.field == "tag":
                unit.tag = value if value else "Untagged"

            elif self.field == "layer":
                unit.layer = value if value else "Default"

            elif self.field == "sprite_name":
                unit.sprite_name = value if value else None

                sprite_renderer = unit.get_component("SpriteRenderer")

                if sprite_renderer:
                    sprite_renderer.sprite_name = unit.sprite_name

            elif self.field == "script":
                unit.script = value if value else None

            elif self.field == "command":
                unit.command = value.upper() if value else "IDLE"

            unit.sync_to_components()

            self.game.history.take_snapshot(f"Edit {self.field}")
            self.game.console.log(f"{self.field} cambiado a {value}", "ENGINE")

        except Exception as error:
            self.game.console.log(f"Error editando {self.field}: {error}", "ERROR")

        self.cancel()

    def handle_click(self, pos):
        if not self.game.selected_units:
            return False

        for field, rect in self.field_rects.items():
            if rect.collidepoint(pos):
                if field.startswith("action:"):
                    action = field.split(":", 1)[1]
                    self.game.inspector_quick_action(action)
                    return True

                unit = self.game.selected_units[0]

                if field.startswith("component:"):
                    _, component_type, attr = field.split(":", 2)
                    component = unit.get_component(component_type)
                    current = getattr(component, attr, "") if component else ""
                else:
                    current = getattr(unit, field, "")

                self.begin_edit(field, current)
                return True

        return False
