class TileLayer:
    def __init__(self, name, width, height, default_tile=-1, opacity=1.0, sorting_order=0):
        self.name = name
        self.width = int(width)
        self.height = int(height)
        self.default_tile = int(default_tile)
        self.opacity = float(opacity)
        self.sorting_order = int(sorting_order)
        self.visible = True
        self.locked = False
        self.collision = False
        self.tiles = [
            [self.default_tile for _ in range(self.width)]
            for _ in range(self.height)
        ]

    def in_bounds(self, x, y):
        return 0 <= int(x) < self.width and 0 <= int(y) < self.height

    def get(self, x, y):
        if not self.in_bounds(x, y):
            return self.default_tile
        return self.tiles[int(y)][int(x)]

    def set(self, x, y, tile):
        if self.locked or not self.in_bounds(x, y):
            return False
        self.tiles[int(y)][int(x)] = int(tile)
        return True

    def erase(self, x, y):
        return self.set(x, y, self.default_tile)

    def fill_rect(self, x, y, width, height, tile):
        changed = 0

        for ty in range(int(y), int(y + height)):
            for tx in range(int(x), int(x + width)):
                if self.set(tx, ty, tile):
                    changed += 1

        return changed

    def used_tiles(self):
        count = 0

        for row in self.tiles:
            for tile in row:
                if tile != self.default_tile:
                    count += 1

        return count

    def resize(self, width, height):
        width = int(width)
        height = int(height)
        new_tiles = [
            [self.default_tile for _ in range(width)]
            for _ in range(height)
        ]

        for y in range(min(self.height, height)):
            for x in range(min(self.width, width)):
                new_tiles[y][x] = self.tiles[y][x]

        self.width = width
        self.height = height
        self.tiles = new_tiles

    def serialize(self):
        return {
            "name": self.name,
            "width": self.width,
            "height": self.height,
            "default_tile": self.default_tile,
            "opacity": self.opacity,
            "sorting_order": self.sorting_order,
            "visible": self.visible,
            "locked": self.locked,
            "collision": self.collision,
            "tiles": self.tiles,
        }

    @classmethod
    def from_data(cls, data, width, height):
        layer = cls(
            data.get("name", "Layer"),
            data.get("width", width),
            data.get("height", height),
            data.get("default_tile", -1),
            data.get("opacity", 1.0),
            data.get("sorting_order", 0),
        )
        layer.visible = data.get("visible", True)
        layer.locked = data.get("locked", False)
        layer.collision = data.get("collision", False)
        layer.tiles = data.get("tiles", layer.tiles)
        layer.resize(width, height)
        return layer


class TilemapLayers:
    DEFAULT_LAYERS = [
        ("Ground", 0, 0.35, False),
        ("Collision", -1, 0.50, True),
        ("Decoration", -1, 0.65, False),
        ("Gameplay", -1, 0.45, False),
    ]

    def __init__(self, width, height):
        self.width = int(width)
        self.height = int(height)
        self.layers = []
        self.active_index = 0
        self.brush_preview = True
        self.autotile_enabled = False

        for order, definition in enumerate(self.DEFAULT_LAYERS):
            name, default_tile, opacity, collision = definition
            layer = TileLayer(name, self.width, self.height, default_tile, opacity, order)
            layer.collision = collision
            self.layers.append(layer)

    @property
    def active_layer(self):
        if not self.layers:
            return None
        self.active_index = max(0, min(self.active_index, len(self.layers) - 1))
        return self.layers[self.active_index]

    def names(self):
        return [layer.name for layer in self.layers]

    def add_layer(self, name=None, default_tile=-1):
        name = name or f"Layer {len(self.layers) + 1}"
        layer = TileLayer(name, self.width, self.height, default_tile, sorting_order=len(self.layers))
        self.layers.append(layer)
        self.active_index = len(self.layers) - 1
        return layer

    def remove_active_layer(self):
        if len(self.layers) <= 1:
            return False
        self.layers.pop(self.active_index)
        self.active_index = max(0, self.active_index - 1)
        return True

    def cycle_layer(self, direction=1):
        if not self.layers:
            return None
        self.active_index = (self.active_index + int(direction)) % len(self.layers)
        return self.active_layer

    def set_tile(self, x, y, tile, layer_name=None):
        layer = self.layer(layer_name) if layer_name else self.active_layer

        if not layer:
            return False

        return layer.set(x, y, tile)

    def erase_tile(self, x, y, layer_name=None):
        layer = self.layer(layer_name) if layer_name else self.active_layer
        return layer.erase(x, y) if layer else False

    def fill_active(self, x, y, width, height, tile):
        layer = self.active_layer
        return layer.fill_rect(x, y, width, height, tile) if layer else 0

    def layer(self, name):
        for layer in self.layers:
            if layer.name == name:
                return layer
        return None

    def toggle_active_visible(self):
        layer = self.active_layer
        if not layer:
            return False
        layer.visible = not layer.visible
        return layer.visible

    def toggle_active_locked(self):
        layer = self.active_layer
        if not layer:
            return False
        layer.locked = not layer.locked
        return layer.locked

    def resize(self, width, height):
        self.width = int(width)
        self.height = int(height)

        for layer in self.layers:
            layer.resize(width, height)

    def stats(self):
        return {
            "layers": len(self.layers),
            "active": self.active_layer.name if self.active_layer else None,
            "used_tiles": sum(layer.used_tiles() for layer in self.layers),
            "collision_tiles": sum(
                layer.used_tiles()
                for layer in self.layers
                if layer.collision
            ),
        }

    def serialize(self):
        return {
            "version": 1,
            "width": self.width,
            "height": self.height,
            "active_index": self.active_index,
            "brush_preview": self.brush_preview,
            "autotile_enabled": self.autotile_enabled,
            "layers": [layer.serialize() for layer in self.layers],
        }

    def deserialize(self, data):
        self.width = int(data.get("width", self.width))
        self.height = int(data.get("height", self.height))
        self.brush_preview = data.get("brush_preview", True)
        self.autotile_enabled = data.get("autotile_enabled", False)
        self.layers = [
            TileLayer.from_data(layer_data, self.width, self.height)
            for layer_data in data.get("layers", [])
        ]

        if not self.layers:
            self.__init__(self.width, self.height)

        self.active_index = max(0, min(data.get("active_index", 0), len(self.layers) - 1))
