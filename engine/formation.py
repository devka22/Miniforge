import math


class Formation:
    """
    Genera posiciones para movimiento en formación.
    """

    @staticmethod
    def square(target_x, target_y, count, spacing=1):
        positions = []

        if count <= 0:
            return positions

        size = math.ceil(math.sqrt(count))

        start_x = target_x - size // 2
        start_y = target_y - size // 2

        for i in range(count):
            x = start_x + (i % size) * spacing
            y = start_y + (i // size) * spacing
            positions.append((int(x), int(y)))

        return positions

    @staticmethod
    def line(target_x, target_y, count, spacing=1):
        positions = []

        start_x = target_x - count // 2

        for i in range(count):
            positions.append((int(start_x + i * spacing), int(target_y)))

        return positions

    @staticmethod
    def column(target_x, target_y, count, spacing=1):
        positions = []

        start_y = target_y - count // 2

        for i in range(count):
            positions.append((int(target_x), int(start_y + i * spacing)))

        return positions

    @staticmethod
    def circle(target_x, target_y, count, radius=2):
        positions = []

        if count <= 0:
            return positions

        for i in range(count):
            angle = (math.pi * 2 * i) / count
            x = target_x + math.cos(angle) * radius
            y = target_y + math.sin(angle) * radius
            positions.append((int(round(x)), int(round(y))))

        return positions

    @staticmethod
    def create(kind, target_x, target_y, count, spacing=1):
        if kind == "line":
            return Formation.line(target_x, target_y, count, spacing)

        if kind == "column":
            return Formation.column(target_x, target_y, count, spacing)

        if kind == "circle":
            return Formation.circle(target_x, target_y, count, max(2, spacing * 2))

        return Formation.square(target_x, target_y, count, spacing)