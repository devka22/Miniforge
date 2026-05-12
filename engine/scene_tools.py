import os
import shutil


class SceneTools:
    """
    Herramientas extra para escenas:
    - Save As
    - Rename
    - Duplicate
    - Delete
    - Backup antes de guardar
    """

    SCENE_FOLDER = "saves/scenes"
    BACKUP_FOLDER = "saves/backups"

    def __init__(self, game):
        self.game = game

        os.makedirs(self.SCENE_FOLDER, exist_ok=True)
        os.makedirs(self.BACKUP_FOLDER, exist_ok=True)

    def current_scene_path(self):
        return os.path.join(
            self.SCENE_FOLDER,
            self.game.scene_manager.current_scene
        )

    def backup_current_scene(self):
        path = self.current_scene_path()

        if not os.path.exists(path):
            return None

        name = self.game.scene_manager.current_scene
        backup_path = os.path.join(
            self.BACKUP_FOLDER,
            name.replace(".scene", "_backup.scene")
        )

        shutil.copy2(path, backup_path)

        self.game.console.log(f"Backup creado: {backup_path}", "SCENE")

        return backup_path

    def save_scene_as(self, new_name):
        from engine.scene_serializer import SceneSerializer

        if not new_name.endswith(".scene"):
            new_name += ".scene"

        path = os.path.join(self.SCENE_FOLDER, new_name)

        SceneSerializer.save(self.game, path)

        self.game.scene_manager.current_scene = new_name
        self.game.scene_manager.refresh()

        self.game.console.log(f"Scene guardada como: {new_name}", "SCENE")

    def rename_current_scene(self, new_name):
        if not new_name.endswith(".scene"):
            new_name += ".scene"

        old_path = self.current_scene_path()
        new_path = os.path.join(self.SCENE_FOLDER, new_name)

        if not os.path.exists(old_path):
            self.game.console.log("No existe la escena actual", "WARNING")
            return

        if os.path.exists(new_path):
            self.game.console.log("Ya existe una escena con ese nombre", "WARNING")
            return

        os.rename(old_path, new_path)

        self.game.scene_manager.current_scene = new_name
        self.game.scene_manager.refresh()

        self.game.console.log(f"Scene renombrada: {new_name}", "SCENE")

    def duplicate_current_scene(self):
        old_path = self.current_scene_path()

        if not os.path.exists(old_path):
            self.game.console.log("No existe la escena actual", "WARNING")
            return

        base = self.game.scene_manager.current_scene.replace(".scene", "")

        index = 1

        while True:
            new_name = f"{base}_copy_{index}.scene"
            new_path = os.path.join(self.SCENE_FOLDER, new_name)

            if not os.path.exists(new_path):
                break

            index += 1

        shutil.copy2(old_path, new_path)

        self.game.scene_manager.refresh()
        self.game.console.log(f"Scene duplicada: {new_name}", "SCENE")

    def delete_current_scene(self):
        path = self.current_scene_path()

        if not os.path.exists(path):
            self.game.console.log("No existe la escena actual", "WARNING")
            return

        self.backup_current_scene()
        os.remove(path)

        self.game.scene_manager.refresh()

        if self.game.scene_manager.scenes:
            self.game.scene_manager.current_scene = self.game.scene_manager.scenes[0]
        else:
            self.game.scene_manager.current_scene = "main.scene"

        self.game.console.log("Scene eliminada", "SCENE")