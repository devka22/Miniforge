# core/engine_config.py

import os

# ============================================================
# MINI VICTORIA ENGINE - CONFIGURACIÓN GLOBAL
# Versión: 0.6.0
# ============================================================

ENGINE_NAME = "Mini Victoria Engine"
ENGINE_VERSION = "0.6.0"

# Carpeta base del proyecto
BASE_DIR = os.getcwd()

# Carpetas principales del proyecto
PROJECT_FOLDERS = [
    "assets",
    "assets/images",
    "assets/audio",
    "assets/fonts",
    "scripts",
    "scenes",
    "logs",
    "config",
    "config/project"
]

# Archivos importantes
PROJECT_FILE = "config/project/project.victoria"
ENGINE_CONFIG_FILE = "config/engine_settings.json"
DEFAULT_SCENE_NAME = "main_scene.json"

# Ventana
WINDOW_WIDTH = 1280
WINDOW_HEIGHT = 720
WINDOW_TITLE = f"{ENGINE_NAME} {ENGINE_VERSION}"
FPS = 60

# Colores base
BACKGROUND_COLOR = (30, 30, 35)
GRID_COLOR = (55, 55, 60)
TEXT_COLOR = (220, 220, 220)

# Debug
DEBUG_MODE = True
SHOW_GRID = True
SHOW_FPS = True

# Formatos soportados
SUPPORTED_IMAGE_FORMATS = [
    ".png",
    ".jpg",
    ".jpeg",
    ".bmp",
    ".gif"
]

SUPPORTED_AUDIO_FORMATS = [
    ".wav",
    ".mp3",
    ".ogg"
]

SUPPORTED_SCRIPT_FORMATS = [
    ".py"
]

SUPPORTED_SCENE_FORMATS = [
    ".json"
]
