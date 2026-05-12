import os
import pygame


class ResourceManager:
    """
    Resource Manager del motor.
    Carga sprites, audio y fuentes.
    """

    def __init__(self, root="assets"):
        self.root = os.path.normpath(root)
        self.images = {}
        self.sounds = {}
        self.fonts = {}
        self.asset_paths = {}
        self.mtimes = {}
        self.missing = set()

        os.makedirs(self.root, exist_ok=True)
        os.makedirs(os.path.join(self.root, "sprites"), exist_ok=True)
        os.makedirs(os.path.join(self.root, "audio"), exist_ok=True)

    def set_root(self, root):
        root = os.path.normpath(root)

        if root == self.root:
            return

        self.root = root
        self.images.clear()
        self.sounds.clear()
        self.asset_paths.clear()
        self.mtimes.clear()

        os.makedirs(self.root, exist_ok=True)
        os.makedirs(os.path.join(self.root, "sprites"), exist_ok=True)
        os.makedirs(os.path.join(self.root, "audio"), exist_ok=True)

    def scan_all(self, project_path=None):
        if project_path:
            self.set_root(os.path.join(project_path, "assets"))

        self.scan_sprites()
        self.scan_audio()

    def scan_sprites(self):
        self.images.clear()

        sprites_path = os.path.join(self.root, "sprites")
        os.makedirs(sprites_path, exist_ok=True)

        for root, _, files in os.walk(sprites_path):
            for filename in files:
                if not filename.lower().endswith((".png", ".jpg", ".jpeg", ".bmp")):
                    continue

                name = os.path.splitext(filename)[0]
                path = os.path.relpath(os.path.join(root, filename), self.root)
                self.load_image(name, path)

    def scan_audio(self):
        self.sounds.clear()

        audio_path = os.path.join(self.root, "audio")
        os.makedirs(audio_path, exist_ok=True)

        for root, _, files in os.walk(audio_path):
            for filename in files:
                if not filename.lower().endswith((".wav", ".ogg", ".mp3")):
                    continue

                name = os.path.splitext(filename)[0]
                path = os.path.relpath(os.path.join(root, filename), self.root)
                self.load_sound(name, path)

    def load_image(self, name, path):
        full_path = os.path.join(self.root, path)

        try:
            image = pygame.image.load(full_path).convert_alpha()
            self.images[name] = image
            self.asset_paths[name] = full_path
            self.mtimes[full_path] = os.path.getmtime(full_path)
            print(f"✅ Sprite cargado: {name}")
            return image
        except Exception as error:
            self.missing.add(name)
            print(f"⚠ No se pudo cargar sprite {name}: {error}")
            return None

    def load_sound(self, name, path):
        full_path = os.path.join(self.root, path)

        try:
            if not pygame.mixer.get_init():
                pygame.mixer.init()

            sound = pygame.mixer.Sound(full_path)
            self.sounds[name] = sound
            self.asset_paths[name] = full_path
            self.mtimes[full_path] = os.path.getmtime(full_path)
            print(f"✅ Audio cargado: {name}")
            return sound
        except Exception as error:
            self.missing.add(name)
            print(f"⚠ No se pudo cargar audio {name}: {error}")
            return None

    def get_image(self, name):
        if not name:
            return None

        return self.images.get(name)

    def get_sound(self, name):
        if not name:
            return None

        return self.sounds.get(name)

    def get_sprite_names(self):
        return list(self.images.keys())

    def get_audio_names(self):
        return list(self.sounds.keys())

    def get_font(self, size):
        if size not in self.fonts:
            self.fonts[size] = pygame.font.SysFont(None, size)

        return self.fonts[size]

    def reload_changed(self):
        changed = 0

        for name, path in list(self.asset_paths.items()):
            if not os.path.exists(path):
                continue

            mtime = os.path.getmtime(path)

            if self.mtimes.get(path) == mtime:
                continue

            relative = os.path.relpath(path, self.root)

            if name in self.images:
                self.load_image(name, relative)
            elif name in self.sounds:
                self.load_sound(name, relative)

            changed += 1

        return changed

    def stats(self):
        return {
            "images": len(self.images),
            "sounds": len(self.sounds),
            "fonts": len(self.fonts),
            "missing": len(self.missing),
        }
