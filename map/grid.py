import random


class Grid:
    """
    Mapa por tiles.
    Valores:
    0 = Grass
    1 = Obstacle
    2 = Sand
    3 = Water
    4 = Stone
    """

    TILE_NAMES = {
        0: "Grass",
        1: "Obstacle",
        2: "Sand",
        3: "Water",
        4: "Stone",
    }

    WALKABLE_TILES = [0, 2, 4]

    def __init__(self, width, height, tile_size, chunk_size=8):
        self.width = width
        self.height = height
        self.tile_size = tile_size
        self.chunk_size = chunk_size

        self.tiles = [[0 for _ in range(width)] for _ in range(height)]

        self.generate_obstacles()
        self.chunks = self.build_chunks()

    def generate_obstacles(self):
        obstacle_count = int(self.width * self.height * 0.04)

        protected = {
            (2, 2),
            (4, 4),
            (6, 6),
            (5, 5),
        }

        for _ in range(obstacle_count):
            x = random.randint(0, self.width - 1)
            y = random.randint(0, self.height - 1)

            if (x, y) in protected:
                continue

            self.tiles[y][x] = 1

    def build_chunks(self):
        chunks = {}

        for y in range(self.height):
            for x in range(self.width):
                cx = x // self.chunk_size
                cy = y // self.chunk_size

                key = (cx, cy)

                if key not in chunks:
                    chunks[key] = []

                chunks[key].append((x, y))

        return chunks

    def get_visible_chunks(self, camera, screen_width, screen_height):
        left_world, top_world = camera.screen_to_world(0, 0)
        right_world, bottom_world = camera.screen_to_world(screen_width, screen_height)

        left_tile = int(left_world // self.tile_size)
        right_tile = int(right_world // self.tile_size) + 1
        top_tile = int(top_world // self.tile_size)
        bottom_tile = int(bottom_world // self.tile_size) + 1

        left_chunk = max(0, left_tile // self.chunk_size)
        right_chunk = min((self.width - 1) // self.chunk_size, right_tile // self.chunk_size)

        top_chunk = max(0, top_tile // self.chunk_size)
        bottom_chunk = min((self.height - 1) // self.chunk_size, bottom_tile // self.chunk_size)

        visible = []

        for cy in range(top_chunk, bottom_chunk + 1):
            for cx in range(left_chunk, right_chunk + 1):
                visible.append((cx, cy))

        return visible

    def is_inside(self, x, y):
        x = int(x)
        y = int(y)

        return 0 <= x < self.width and 0 <= y < self.height

    def is_walkable(self, x, y):
        x = int(x)
        y = int(y)

        if not self.is_inside(x, y):
            return False

        return self.tiles[y][x] in self.WALKABLE_TILES

    def set_tile(self, x, y, value):
        x = int(x)
        y = int(y)

        if not self.is_inside(x, y):
            return

        self.tiles[y][x] = int(value)

    def get_tile(self, x, y):
        x = int(x)
        y = int(y)

        if not self.is_inside(x, y):
            return 0

        return self.tiles[y][x]

    def toggle_obstacle(self, x, y):
        x = int(x)
        y = int(y)

        if not self.is_inside(x, y):
            return

        self.tiles[y][x] = 0 if self.tiles[y][x] == 1 else 1

    def nearest_walkable(self, x, y, max_radius=8):
        x = int(x)
        y = int(y)

        if self.is_walkable(x, y):
            return x, y

        for radius in range(1, max_radius + 1):
            for yy in range(y - radius, y + radius + 1):
                for xx in range(x - radius, x + radius + 1):
                    if self.is_walkable(xx, yy):
                        return xx, yy

        return 0, 0

    def serialize(self):
        return {
            "width": self.width,
            "height": self.height,
            "tile_size": self.tile_size,
            "chunk_size": self.chunk_size,
            "tiles": self.tiles
        }

    def deserialize(self, data):
        self.width = data.get("width", self.width)
        self.height = data.get("height", self.height)
        self.tile_size = data.get("tile_size", self.tile_size)
        self.chunk_size = data.get("chunk_size", self.chunk_size)
        self.tiles = data.get("tiles", self.tiles)
        self.chunks = self.build_chunks()