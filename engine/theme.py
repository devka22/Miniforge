class Theme:
    """
    Tema visual centralizado. Mantiene una UI más consistente.
    """

    def __init__(self):
        self.data = {
            "bg": (238, 240, 245),
            "panel": (250, 250, 252),
            "panel_alt": (245, 247, 252),
            "border": (198, 203, 216),
            "text": (35, 36, 42),
            "muted": (90, 94, 108),
            "primary": (0, 122, 255),
            "success": (35, 150, 90),
            "warning": (210, 145, 55),
            "danger": (210, 70, 70),
        }

    def get(self, key, default=None):
        return self.data.get(key, default)
