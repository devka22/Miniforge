import os
import shutil


class AssetOperations:
    """
    Operaciones del Content Browser:
    - Crear carpeta
    - Renombrar asset
    - Borrar asset
    """

    def __init__(self, game):
        self.game = game

    def create_folder(self, base="assets", name=None):
        if name is None:
            index = 1

            while True:
                name = f"NewFolder_{index}"
                path = os.path.join(base, name)

                if not os.path.exists(path):
                    break

                index += 1
        else:
            path = os.path.join(base, name)

        os.makedirs(path, exist_ok=True)

        self.game.refresh_project()
        self.game.console.log(f"Carpeta creada: {path}", "ASSET")

    def rename_selected_asset(self, new_name):
        asset = self.game.file_browser.selected_asset

        if not asset:
            self.game.console.log("No hay asset seleccionado", "WARNING")
            return

        old_path = asset["path"]
        folder = os.path.dirname(old_path)
        ext = os.path.splitext(old_path)[1]

        if not new_name.endswith(ext):
            new_name += ext

        new_path = os.path.join(folder, new_name)

        if os.path.exists(new_path):
            self.game.console.log("Ya existe un asset con ese nombre", "WARNING")
            return

        os.rename(old_path, new_path)

        self.game.refresh_project()
        self.game.console.log(f"Asset renombrado: {new_name}", "ASSET")

    def delete_selected_asset(self):
        asset = self.game.file_browser.selected_asset

        if not asset:
            self.game.console.log("No hay asset seleccionado", "WARNING")
            return

        path = asset["path"]

        if not os.path.exists(path):
            self.game.console.log("El asset no existe", "WARNING")
            return

        trash_folder = "project/deleted_assets"
        os.makedirs(trash_folder, exist_ok=True)

        target = os.path.join(trash_folder, os.path.basename(path))

        if os.path.exists(target):
            os.remove(target)

        shutil.move(path, target)

        self.game.file_browser.selected_asset = None
        self.game.refresh_project()

        self.game.console.log(f"Asset enviado a papelera interna: {target}", "ASSET")