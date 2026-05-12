import os
import re


SCRIPT_TEMPLATE = '''from engine.script import Script


class {class_name}(Script):
    script_name = "{class_name}"

    def __init__(self):
        super().__init__()

    def start(self, entity):
        # Se ejecuta una vez cuando el script inicia
        pass

    def update(self, entity, dt):
        # Se ejecuta cada frame en modo PLAY
        # ejemplo:
        # entity.x += dt * 2
        pass

    def on_selected(self, entity):
        # Se ejecuta cuando seleccionas la entidad
        pass

    def on_deselected(self, entity):
        # Se ejecuta cuando deseleccionas la entidad
        pass
'''


class ProjectBrowser:
    """
    Mini Content Browser del motor.
    Maneja carpetas, scripts y selección de assets.
    """

    def __init__(self, game):
        self.game = game

        self.selected_script_name = None
        self.selected_sprite_name = None

        self.refresh()

    def refresh(self):
        os.makedirs("assets/sprites", exist_ok=True)
        os.makedirs("scripts", exist_ok=True)

        init_file = "scripts/__init__.py"
        if not os.path.exists(init_file):
            with open(init_file, "w") as f:
                f.write("# scripts package\n")

        self.sprite_files = []
        self.script_files = []
        self.folders = []

        for root, dirs, files in os.walk("assets"):
            for d in dirs:
                self.folders.append(os.path.join(root, d))

        for file in os.listdir("assets/sprites"):
            if file.lower().endswith((".png", ".jpg", ".jpeg")):
                self.sprite_files.append(file)

        for file in os.listdir("scripts"):
            if file.endswith(".py") and file != "__init__.py":
                self.script_files.append(file)

    def make_class_name(self, filename):
        base = os.path.splitext(filename)[0]
        parts = re.split(r"[^a-zA-Z0-9]+", base)

        class_name = "".join(p.capitalize() for p in parts if p)

        if not class_name:
            class_name = "NewScript"

        if class_name[0].isdigit():
            class_name = "Script" + class_name

        return class_name

    def create_script(self):
        os.makedirs("scripts", exist_ok=True)

        index = 1

        while True:
            filename = f"user_script_{index}.py"
            path = os.path.join("scripts", filename)

            if not os.path.exists(path):
                break

            index += 1

        class_name = self.make_class_name(filename)

        with open(path, "w") as f:
            f.write(SCRIPT_TEMPLATE.format(class_name=class_name))

        self.game.console.log(f"Script creado: {filename}")

        self.refresh()
        self.game.script_manager.scan_scripts()

        self.selected_script_name = class_name
        self.game.script_editor.open_file(path)

    def create_folder(self):
        os.makedirs("assets", exist_ok=True)

        index = 1

        while True:
            path = os.path.join("assets", f"folder_{index}")

            if not os.path.exists(path):
                break

            index += 1

        os.makedirs(path, exist_ok=True)

        self.game.console.log(f"Carpeta creada: {path}")
        self.refresh()

    def next_script(self):
        names = self.game.script_manager.get_script_names()

        if not names:
            self.selected_script_name = None
            self.game.console.log("No hay scripts disponibles")
            return

        if self.selected_script_name not in names:
            self.selected_script_name = names[0]
        else:
            index = names.index(self.selected_script_name)
            self.selected_script_name = names[(index + 1) % len(names)]

        self.game.console.log(f"Script seleccionado: {self.selected_script_name}")

    def next_sprite(self):
        sprites = self.game.resources.get_sprite_names()

        if not sprites:
            self.selected_sprite_name = None
            self.game.console.log("No hay sprites disponibles")
            return

        if self.selected_sprite_name not in sprites:
            self.selected_sprite_name = sprites[0]
        else:
            index = sprites.index(self.selected_sprite_name)
            self.selected_sprite_name = sprites[(index + 1) % len(sprites)]

        self.game.console.log(f"Sprite seleccionado: {self.selected_sprite_name}")

    def open_selected_script(self):
        if not self.selected_script_name:
            self.game.console.log("No hay script seleccionado")
            return

        script_file = self.find_script_file_for_class(self.selected_script_name)

        if not script_file:
            self.game.console.log("No se encontró el archivo del script")
            return

        self.game.script_editor.open_file(script_file)
        self.game.script_editor.visible = True

    def find_script_file_for_class(self, class_name):
        for file in self.script_files:
            path = os.path.join("scripts", file)

            try:
                with open(path, "r") as f:
                    content = f.read()

                if f"class {class_name}" in content:
                    return path

            except Exception:
                pass

        return None