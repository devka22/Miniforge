class CreateAssetModal:
    """
    MiniForge 0.6.0
    Modal para crear y renombrar archivos desde el motor.

    Sirve para:
    - crear scripts
    - crear componentes
    - crear sistemas
    - crear escenas
    - crear prefabs
    - crear JSON
    - crear TXT
    - crear carpetas
    - renombrar assets
    - renombrar carpetas
    """

    def __init__(self, game):
        self.game = game

        self.visible = False
        self.mode = None
        self.title = ""
        self.placeholder = ""
        self.buffer = ""
        self.target = None

    def open(self, mode, title, placeholder="", target=None):
        self.visible = True
        self.mode = mode
        self.title = title
        self.placeholder = placeholder
        self.buffer = ""
        self.target = target

        if hasattr(self.game, "console"):
            self.game.console.log(f"Modal abierto: {title}", "EDITOR")

    def close(self):
        self.visible = False
        self.mode = None
        self.title = ""
        self.placeholder = ""
        self.buffer = ""
        self.target = None

    def handle_key(self, event):
        import pygame

        if not self.visible:
            return False

        if event.key == pygame.K_ESCAPE:
            self.close()
            return True

        if event.key == pygame.K_RETURN:
            self.confirm()
            return True

        if event.key == pygame.K_BACKSPACE:
            self.buffer = self.buffer[:-1]
            return True

        if event.unicode and event.unicode.isprintable():
            self.buffer += event.unicode
            return True

        return True

    def confirm(self):
        name = self.buffer.strip()

        if not name:
            name = self.placeholder or "NewAsset"

        fb = self.game.file_browser

        if self.mode == "create_script":
            fb.create_script(name)

        elif self.mode == "create_component":
            fb.create_component(name)

        elif self.mode == "create_system":
            fb.create_system(name)

        elif self.mode == "create_scene":
            fb.create_scene(name)

        elif self.mode == "create_prefab":
            fb.create_prefab(name)

        elif self.mode == "create_json":
            fb.create_json(name)

        elif self.mode == "create_txt":
            fb.create_txt(name)

        elif self.mode == "create_folder":
            fb.create_folder(name)

        elif self.mode == "rename_asset":
            fb.rename_selected_asset(name)

        elif self.mode == "rename_folder":
            fb.rename_selected_folder(name)

        else:
            if hasattr(self.game, "console"):
                self.game.console.log(
                    f"Modo modal desconocido: {self.mode}",
                    "WARNING"
                )

        if hasattr(self.game, "refresh_project"):
            self.game.refresh_project()

        self.close()