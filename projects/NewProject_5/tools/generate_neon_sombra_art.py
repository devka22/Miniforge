#!/usr/bin/env python3
from __future__ import annotations

import json
import math
import struct
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WIDTH = 128
HEIGHT = 80
TILE = 16

CITY_BACKDROP = "assets/sprites/neon_sombra_city_backdrop.png"
SPRITE_ATLAS = "assets/sprites/neon_sombra_atlas.png"
SPRITE_MANIFEST = "assets/sprites/neon_sombra_atlas.json"

BUILDINGS = [
    (6, 5, 6, 5, "Taller_Azul", [74, 88, 110]),
    (18, 5, 6, 5, "Deposito_Mate", [88, 78, 67]),
    (31, 7, 7, 5, "Garaje_Once", [102, 86, 71]),
    (8, 69, 7, 5, "Pesquera_Gris", [58, 86, 92]),
    (22, 69, 8, 5, "Hangar_Muelle", [76, 88, 86]),
    (37, 69, 8, 5, "Mercado_Nocturno", [124, 82, 96]),
    (53, 69, 8, 5, "Terminal_Bus", [88, 92, 116]),
    (68, 69, 8, 5, "Oficinas_Radar", [76, 101, 123]),
    (83, 69, 8, 5, "Plaza_Eclipse", [93, 88, 128]),
    (5, 17, 7, 6, "Fabrica_Lumen", [94, 84, 94]),
    (18, 17, 6, 6, "Taller_Ivo", [110, 81, 71]),
    (31, 18, 7, 5, "Astillero_Seco", [72, 92, 104]),
    (31, 28, 5, 3, "Lavanderia_Neon", [78, 86, 106]),
    (46, 3, 5, 3, "Hostal_Lira", [85, 93, 118]),
    (58, 4, 5, 3, "Farmacia_24", [76, 106, 112]),
    (45, 13, 8, 5, "Banco_Cobalto", [81, 96, 122]),
    (59, 14, 7, 7, "Torre_Notarial", [92, 99, 118]),
    (45, 26, 8, 5, "Centro_Civico", [105, 112, 130]),
    (59, 27, 7, 5, "Archivo_Publico", [86, 98, 116]),
    (73, 8, 8, 5, "Mirador_Hotel", [100, 95, 124]),
    (88, 9, 6, 6, "Club_Azul", [91, 73, 117]),
    (73, 22, 8, 5, "Residencial_Norte", [80, 102, 112]),
    (73, 27, 5, 3, "Oficina_Marea", [83, 96, 125]),
    (88, 23, 6, 5, "Clinica_Luna", [97, 114, 116]),
    (88, 37, 5, 3, "Galeria_Roja", [117, 78, 108]),
    (6, 41, 7, 5, "Lonja_Puerto", [66, 97, 105]),
    (19, 41, 7, 6, "Aduana_Roja", [118, 77, 82]),
    (7, 54, 8, 5, "Muelle_17", [57, 82, 100]),
    (22, 55, 7, 5, "Bodega_Sal", [75, 92, 86]),
    (44, 41, 8, 5, "Mercado_Viejo", [121, 86, 86]),
    (58, 42, 7, 5, "Cine_Aurora", [108, 78, 111]),
    (31, 50, 5, 3, "Kiosko_Radio", [107, 83, 66]),
    (44, 55, 8, 5, "Pasaje_Granate", [118, 83, 94]),
    (59, 55, 7, 5, "Casa_Radio", [87, 101, 118]),
    (74, 41, 7, 6, "Ribera_Este", [73, 108, 106]),
    (89, 42, 7, 5, "Teatro_Ribera", [114, 92, 120]),
    (74, 55, 7, 5, "Jardin_Luces", [80, 118, 88]),
    (89, 55, 6, 5, "Comisaria_Norte", [82, 97, 128]),
    (101, 8, 7, 5, "Club_Pearl", [103, 75, 122]),
    (114, 9, 8, 6, "Hotel_Bahia", [116, 103, 132]),
    (123, 14, 6, 5, "Parking_Coral", [85, 87, 102]),
    (101, 23, 7, 5, "Plaza_Marina", [97, 105, 124]),
    (114, 24, 8, 6, "Condos_Sol", [121, 110, 125]),
    (123, 30, 6, 5, "Clinica_Bahia", [92, 121, 124]),
    (100, 40, 7, 5, "Ocean_Drive", [102, 84, 121]),
    (112, 39, 7, 5, "Disco_Coral", [129, 69, 117]),
    (123, 43, 6, 5, "Cafe_Muelle", [125, 91, 83]),
    (100, 55, 7, 5, "Pier_21_Entrada", [114, 86, 70]),
    (111, 57, 7, 5, "Tienda_Surf", [90, 122, 119]),
    (123, 58, 6, 5, "Yacht_Club", [82, 104, 130]),
    (101, 72, 8, 5, "Hotel_Orquidea", [130, 96, 118]),
    (116, 72, 9, 5, "Apartamentos_Neon", [92, 108, 134]),
]

SPRITES = {
    "Player": [0, 0, 32, 32],
    "Pedestrian": [32, 0, 32, 32],
    "Officer": [64, 0, 32, 32],
    "Mara": [96, 0, 32, 32],
    "Ivo": [128, 0, 32, 32],
    "Valeria": [160, 0, 32, 32],
    "Nodo de senal": [192, 0, 32, 32],
    "Car": [0, 40, 48, 32],
    "PoliceCar": [48, 40, 48, 32],
    "Lamp": [104, 40, 16, 24],
    "Pickup": [128, 40, 16, 16],
    "Roadblock": [152, 40, 64, 16],
    "SavePoint": [220, 40, 24, 24],
    "Yacht": [0, 76, 56, 24],
}


class Canvas:
    def __init__(self, width: int, height: int, color: tuple[int, int, int, int]) -> None:
        self.width = width
        self.height = height
        self.pixels = bytearray(color * (width * height))

    def set_px(self, x: int, y: int, color: tuple[int, int, int, int]) -> None:
        if x < 0 or y < 0 or x >= self.width or y >= self.height:
            return
        offset = (y * self.width + x) * 4
        self.pixels[offset : offset + 4] = bytes(color)

    def rect(self, x: int, y: int, width: int, height: int, color: tuple[int, int, int, int]) -> None:
        x0 = max(0, x)
        y0 = max(0, y)
        x1 = min(self.width, x + width)
        y1 = min(self.height, y + height)
        if x0 >= x1 or y0 >= y1:
            return
        row = bytes(color) * (x1 - x0)
        for py in range(y0, y1):
            offset = (py * self.width + x0) * 4
            self.pixels[offset : offset + len(row)] = row

    def outline(self, x: int, y: int, width: int, height: int, color: tuple[int, int, int, int], thickness: int = 1) -> None:
        self.rect(x, y, width, thickness, color)
        self.rect(x, y + height - thickness, width, thickness, color)
        self.rect(x, y, thickness, height, color)
        self.rect(x + width - thickness, y, thickness, height, color)

    def circle(self, cx: int, cy: int, radius: int, color: tuple[int, int, int, int]) -> None:
        radius_sq = radius * radius
        for py in range(cy - radius, cy + radius + 1):
            for px in range(cx - radius, cx + radius + 1):
                dx = px - cx
                dy = py - cy
                if dx * dx + dy * dy <= radius_sq:
                    self.set_px(px, py, color)


def save_png(path: Path, canvas: Canvas) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    raw = bytearray()
    stride = canvas.width * 4
    for y in range(canvas.height):
        raw.append(0)
        start = y * stride
        raw.extend(canvas.pixels[start : start + stride])

    def chunk(kind: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + kind
            + data
            + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
        )

    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", canvas.width, canvas.height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )
    path.write_bytes(png)


def shade(color: list[int], delta: int, alpha: int = 255) -> tuple[int, int, int, int]:
    return tuple(max(0, min(255, channel + delta)) for channel in color) + (alpha,)


def district_color(x: int, y: int) -> list[int]:
    if x >= 116 and y >= 28:
        return [18, 54, 76]
    if x >= 96 and y >= 34:
        return [92, 78, 58]
    if y >= 64:
        return [45, 52, 62]
    if x < 31 and y > 35:
        return [28, 58, 61]
    if x < 32 and y < 31:
        return [53, 48, 43]
    if 34 <= x <= 66 and 34 <= y:
        return [61, 45, 65]
    if x > 68 and y > 31:
        return [44, 58, 52]
    if x > 96 and y < 34:
        return [61, 54, 76]
    if x > 63 and y < 30:
        return [48, 55, 70]
    return [34, 44, 58]


def draw_road_vertical(canvas: Canvas, tile_x: int) -> None:
    px = int((tile_x - 1.55) * TILE)
    width = int(3.1 * TILE)
    canvas.rect(px - 3, 0, width + 6, HEIGHT * TILE, (21, 28, 38, 255))
    canvas.rect(px, 0, width, HEIGHT * TILE, (34, 42, 51, 255))
    canvas.rect(px, 0, 2, HEIGHT * TILE, (68, 226, 224, 150))
    canvas.rect(px + width - 2, 0, 2, HEIGHT * TILE, (250, 132, 206, 135))
    for y in range(10, HEIGHT * TILE, 34):
        canvas.rect(px + width // 2 - 1, y, 2, 14, (201, 190, 142, 180))


def draw_road_horizontal(canvas: Canvas, tile_y: int) -> None:
    py = int((tile_y - 1.55) * TILE)
    height = int(3.1 * TILE)
    canvas.rect(0, py - 3, WIDTH * TILE, height + 6, (21, 28, 38, 255))
    canvas.rect(0, py, WIDTH * TILE, height, (34, 42, 51, 255))
    canvas.rect(0, py, WIDTH * TILE, 2, (68, 226, 224, 145))
    canvas.rect(0, py + height - 2, WIDTH * TILE, 2, (250, 132, 206, 130))
    for x in range(12, WIDTH * TILE, 34):
        canvas.rect(x, py + height // 2 - 1, 14, 2, (201, 190, 142, 180))


def draw_building(canvas: Canvas, bx: float, by: float, bw: float, bh: float, name: str, tint: list[int]) -> None:
    x = int((bx - bw * 0.5) * TILE)
    y = int((by - bh * 0.5) * TILE)
    width = int(bw * TILE)
    height = int(bh * TILE)
    canvas.rect(x + 7, y + 9, width + 5, height + 5, (8, 11, 17, 170))
    canvas.rect(x + width, y + 8, 6, height, shade(tint, -42))
    canvas.rect(x + 8, y + height, width, 6, shade(tint, -50))
    canvas.rect(x, y, width, height, shade(tint, -8))
    canvas.rect(x + 5, y + 5, width - 10, height - 10, shade(tint, 8))
    canvas.outline(x, y, width, height, (9, 16, 27, 255), 2)
    neon = (93, 238, 255, 210) if sum(tint) % 2 == 0 else (255, 114, 208, 205)
    canvas.rect(x + 4, y + 3, width - 8, 2, neon)
    canvas.rect(x + 3, y + height - 6, width - 6, 2, (255, 219, 121, 145))
    seed = sum(ord(ch) for ch in name)
    for wy in range(y + 12, y + height - 10, 15):
        for wx in range(x + 10, x + width - 8, 16):
            if (wx + wy + seed) % 4 == 0:
                canvas.rect(wx, wy, 5, 7, (255, 221, 142, 190))
            else:
                canvas.rect(wx, wy, 5, 7, (20, 27, 39, 210))
    if width > 72:
        canvas.rect(x + width // 2 - 12, y + height // 2 - 8, 24, 16, shade(tint, -24))
        canvas.outline(x + width // 2 - 12, y + height // 2 - 8, 24, 16, (12, 18, 26, 230), 1)


def draw_beachfront(canvas: Canvas) -> None:
    water = (23, 62, 78, 255)
    water_dark = (13, 38, 58, 255)
    sand = (119, 101, 70, 255)
    sand_light = (165, 139, 88, 255)
    water_x = 116 * TILE
    canvas.rect(water_x, 28 * TILE, WIDTH * TILE - water_x, 52 * TILE, water)
    for y in range(28 * TILE, HEIGHT * TILE, 18):
        canvas.rect(water_x + 4 + (y // 18) % 9, y, WIDTH * TILE - water_x - 8, 2, (82, 188, 199, 90))
        canvas.rect(water_x + 18 + (y // 11) % 15, y + 7, WIDTH * TILE - water_x - 30, 1, (142, 236, 241, 48))
    canvas.rect(94 * TILE, 34 * TILE, 22 * TILE, 31 * TILE, sand)
    canvas.rect(94 * TILE, 65 * TILE, 22 * TILE, 9 * TILE, (88, 77, 61, 255))
    for y in range(35 * TILE, 74 * TILE, 11):
        canvas.rect(95 * TILE + 5, y, 20 * TILE - 8, 1, sand_light)
    canvas.rect(92 * TILE, 34 * TILE, 2 * TILE, 40 * TILE, (45, 39, 35, 255))
    canvas.rect(92 * TILE + 5, 34 * TILE, 2, 40 * TILE, (88, 74, 55, 220))
    for ty in [38, 43, 48, 53, 58, 66, 71]:
        px = 97 * TILE
        py = ty * TILE
        canvas.rect(px - 2, py - 7, 4, 15, (83, 58, 35, 255))
        for angle in range(0, 360, 60):
            dx = int(math.cos(math.radians(angle)) * 9)
            dy = int(math.sin(math.radians(angle)) * 6)
            canvas.rect(px + dx - 2, py + dy - 2, 5, 5, (48, 139, 91, 230))
    for tx, ty, color in [
        (101, 40, (255, 103, 188, 230)),
        (104, 46, (86, 231, 255, 230)),
        (102, 52, (255, 215, 97, 230)),
        (107, 57, (255, 103, 188, 230)),
        (103, 68, (86, 231, 255, 230)),
        (111, 70, (255, 215, 97, 230)),
    ]:
        px = tx * TILE
        py = ty * TILE
        canvas.rect(px - 8, py - 4, 16, 8, color)
        canvas.rect(px - 1, py + 4, 2, 7, (58, 42, 28, 255))

    pier_y = 59 * TILE
    canvas.rect(108 * TILE, pier_y - 12, 18 * TILE, 24, (75, 57, 42, 255))
    for x in range(108 * TILE, 126 * TILE, 20):
        canvas.rect(x, pier_y - 17, 5, 34, (43, 31, 25, 255))
        canvas.rect(x + 2, pier_y - 20, 2, 40, (121, 95, 65, 220))
    canvas.rect(110 * TILE, pier_y - 4, 14 * TILE, 3, (255, 95, 196, 210))
    canvas.rect(110 * TILE, pier_y + 5, 14 * TILE, 3, (87, 235, 255, 210))
    canvas.rect(112 * TILE, 55 * TILE, 84, 18, (28, 24, 28, 230))
    canvas.rect(116 * TILE, 55 * TILE + 4, 54, 4, (255, 99, 203, 210))
    canvas.rect(116 * TILE, 55 * TILE + 10, 54, 3, (88, 236, 255, 210))


def draw_yacht_on_backdrop(canvas: Canvas, tx: float, ty: float, tint: tuple[int, int, int, int]) -> None:
    px = int(tx * TILE)
    py = int(ty * TILE)
    canvas.rect(px - 24, py - 4, 48, 10, (8, 17, 24, 170))
    canvas.rect(px - 22, py - 8, 44, 15, tint)
    canvas.rect(px - 12, py - 14, 24, 8, (210, 234, 240, 245))
    canvas.rect(px - 2, py - 22, 3, 13, (50, 57, 66, 255))
    canvas.rect(px + 2, py - 21, 16, 9, (235, 238, 230, 210))
    canvas.rect(px - 20, py + 7, 40, 2, (94, 232, 255, 170))
    canvas.outline(px - 23, py - 9, 46, 17, (5, 9, 14, 255), 1)


def draw_city_backdrop() -> None:
    canvas = Canvas(WIDTH * TILE, HEIGHT * TILE, (13, 18, 27, 255))
    for ty in range(HEIGHT):
        for tx in range(WIDTH):
            base = district_color(tx, ty)
            noise = ((tx * 17 + ty * 29) % 7) - 3
            canvas.rect(tx * TILE, ty * TILE, TILE, TILE, shade(base, noise))
            if (tx + ty) % 9 == 0:
                canvas.rect(tx * TILE + 2, ty * TILE + 11, 3, 2, shade(base, 18, 150))

    road_columns = [10, 24, 38, 52, 66, 80, 96, 112]
    road_rows = [8, 20, 32, 44, 56, 68]
    for x in road_columns:
        draw_road_vertical(canvas, x)
    for y in road_rows:
        draw_road_horizontal(canvas, y)
    draw_beachfront(canvas)
    for x in road_columns:
        for y in road_rows:
            canvas.rect((x - 2) * TILE, (y - 2) * TILE, 4 * TILE, 4 * TILE, (38, 45, 54, 255))
            for stripe in range(-18, 20, 8):
                canvas.rect(x * TILE + stripe, y * TILE - 22, 2, 12, (230, 227, 194, 130))
                canvas.rect(x * TILE - 22, y * TILE + stripe, 12, 2, (230, 227, 194, 130))

    for building in BUILDINGS:
        draw_building(canvas, *building)

    for px in range(0, 32 * TILE, 28):
        canvas.rect(px, 50 * TILE + (px // 28) % 3 * 9, 18, 4, (98, 116, 112, 210))
        canvas.rect(px + 4, 51 * TILE + (px // 28) % 3 * 9, 4, 46, (62, 79, 86, 210))
    for index, (tx, ty) in enumerate([(76, 46), (82, 50), (88, 48), (74, 57), (90, 57), (70, 36), (101, 63), (109, 66)]):
        px = tx * TILE
        py = ty * TILE
        canvas.rect(px - 2, py - 7, 4, 16, (81, 64, 45, 255))
        for angle in range(0, 360, 45):
            dx = int(math.cos(math.radians(angle)) * 10)
            dy = int(math.sin(math.radians(angle)) * 10)
            canvas.rect(px + dx - 2, py + dy - 2, 5, 5, (52, 128 + index * 5 % 40, 91, 230))

    for yacht in [(121.5, 52.5, (226, 236, 238, 255)), (123.0, 61.5, (238, 220, 232, 255)), (119.5, 70.0, (218, 234, 226, 255))]:
        draw_yacht_on_backdrop(canvas, yacht[0], yacht[1], yacht[2])

    save_png(ROOT / CITY_BACKDROP, canvas)


def draw_person(canvas: Canvas, x: int, y: int, coat: tuple[int, int, int, int], accent: tuple[int, int, int, int]) -> None:
    canvas.rect(x + 13, y + 4, 6, 5, (31, 24, 22, 255))
    canvas.rect(x + 11, y + 9, 10, 11, coat)
    canvas.rect(x + 14, y + 20, 4, 8, shade(list(coat[:3]), -28))
    canvas.rect(x + 8, y + 11, 4, 10, shade(list(coat[:3]), -22))
    canvas.rect(x + 21, y + 11, 4, 10, shade(list(coat[:3]), -22))
    canvas.rect(x + 12, y + 10, 8, 2, accent)
    canvas.outline(x + 10, y + 8, 12, 20, (5, 9, 15, 255), 1)


def draw_car(canvas: Canvas, x: int, y: int, body: tuple[int, int, int, int], police: bool = False) -> None:
    canvas.rect(x + 4, y + 8, 40, 16, (7, 10, 14, 255))
    canvas.rect(x + 6, y + 6, 36, 20, body)
    canvas.rect(x + 15, y + 8, 16, 5, (58, 78, 94, 255))
    canvas.rect(x + 15, y + 19, 16, 5, (41, 57, 70, 255))
    canvas.rect(x + 3, y + 9, 5, 5, (247, 225, 154, 255))
    canvas.rect(x + 40, y + 9, 5, 5, (255, 72, 80, 255))
    canvas.rect(x + 40, y + 18, 5, 5, (255, 72, 80, 255))
    if police:
        canvas.rect(x + 22, y + 14, 4, 4, (92, 170, 255, 255))
        canvas.rect(x + 26, y + 14, 4, 4, (255, 78, 110, 255))
        canvas.rect(x + 7, y + 15, 35, 2, (245, 247, 255, 230))
    canvas.outline(x + 5, y + 5, 38, 22, (4, 8, 13, 255), 1)


def draw_yacht_sprite(canvas: Canvas, x: int, y: int) -> None:
    canvas.rect(x + 4, y + 13, 48, 5, (4, 9, 16, 145))
    canvas.rect(x + 6, y + 7, 44, 13, (223, 235, 238, 255))
    canvas.rect(x + 15, y + 3, 20, 7, (119, 203, 219, 255))
    canvas.rect(x + 27, y + 0, 3, 14, (35, 44, 54, 255))
    canvas.rect(x + 31, y + 2, 14, 8, (246, 244, 224, 230))
    canvas.rect(x + 8, y + 18, 40, 2, (76, 230, 255, 180))
    canvas.rect(x + 12, y + 11, 5, 3, (255, 99, 196, 230))
    canvas.rect(x + 39, y + 11, 5, 3, (255, 217, 112, 230))
    canvas.outline(x + 5, y + 6, 46, 15, (3, 7, 12, 255), 1)


def draw_sprite_atlas() -> None:
    canvas = Canvas(256, 128, (0, 0, 0, 0))
    draw_person(canvas, 0, 0, (55, 176, 226, 255), (255, 96, 194, 255))
    draw_person(canvas, 32, 0, (112, 152, 126, 255), (255, 211, 122, 255))
    draw_person(canvas, 64, 0, (62, 110, 214, 255), (144, 204, 255, 255))
    draw_person(canvas, 96, 0, (222, 141, 86, 255), (255, 231, 168, 255))
    draw_person(canvas, 128, 0, (198, 92, 76, 255), (255, 188, 95, 255))
    draw_person(canvas, 160, 0, (118, 177, 219, 255), (236, 247, 255, 255))
    canvas.rect(203, 4, 10, 24, (123, 78, 214, 255))
    canvas.rect(199, 8, 18, 4, (95, 232, 255, 230))
    canvas.rect(198, 24, 20, 3, (255, 115, 214, 220))
    canvas.outline(201, 3, 14, 27, (7, 9, 18, 255), 1)

    draw_car(canvas, 0, 40, (216, 171, 86, 255), False)
    draw_car(canvas, 48, 40, (73, 117, 220, 255), True)
    canvas.rect(110, 42, 4, 18, (24, 28, 36, 255))
    canvas.rect(105, 40, 14, 4, (255, 225, 135, 255))
    canvas.circle(112, 40, 4, (255, 218, 130, 210))
    canvas.rect(130, 42, 12, 12, (78, 230, 255, 255))
    canvas.rect(133, 45, 6, 6, (255, 255, 255, 255))
    canvas.outline(129, 41, 14, 14, (5, 9, 15, 255), 1)
    canvas.rect(152, 45, 64, 8, (30, 44, 72, 255))
    for x in range(156, 214, 12):
        canvas.rect(x, 42, 4, 14, (108, 177, 255, 255))
        canvas.rect(x + 5, 42, 3, 14, (255, 87, 122, 255))
    canvas.outline(152, 44, 64, 10, (5, 9, 15, 255), 1)
    canvas.circle(232, 53, 13, (95, 238, 255, 80))
    canvas.rect(223, 44, 18, 18, (23, 36, 54, 255))
    canvas.rect(226, 47, 12, 12, (255, 222, 112, 255))
    canvas.rect(228, 49, 8, 3, (13, 30, 47, 255))
    canvas.rect(228, 54, 8, 3, (13, 30, 47, 255))
    canvas.outline(222, 43, 20, 20, (4, 8, 13, 255), 1)
    draw_yacht_sprite(canvas, 0, 76)

    save_png(ROOT / SPRITE_ATLAS, canvas)
    manifest = {
        "atlas": SPRITE_ATLAS,
        "city_backdrop": CITY_BACKDROP,
        "sprites": {
            name: {"x": rect[0], "y": rect[1], "width": rect[2], "height": rect[3]}
            for name, rect in SPRITES.items()
        },
    }
    (ROOT / SPRITE_MANIFEST).write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def write_assets() -> None:
    draw_city_backdrop()
    draw_sprite_atlas()


if __name__ == "__main__":
    write_assets()
