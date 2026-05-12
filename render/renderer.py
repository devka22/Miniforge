import os
import json
import pygame


class Renderer:
    """
    MiniForge Renderer 0.6.0 Alpha.

    Renderiza:
    - mundo / grid / tiles
    - entidades
    - selección
    - gizmos
    - navigator
    - content browser
    - quick buttons del browser
    - context menu
    - create/rename modal
    - scene hierarchy
    - minimap
    - inspector
    - settings panels
    - console
    - script editor
    - mensajes visuales
    """

    def __init__(self, game):
        self.game = game

        self.font = pygame.font.SysFont(None, 22)
        self.small_font = pygame.font.SysFont(None, 18)
        self.tiny_font = pygame.font.SysFont(None, 15)
        self.code_font = pygame.font.SysFont("Menlo", 16)

        theme = getattr(game, "theme", None)
        self.bg_color = theme.get("bg", (242, 243, 247)) if theme else (242, 243, 247)
        self.panel_color = theme.get("panel", (250, 250, 252)) if theme else (250, 250, 252)
        self.panel_border = theme.get("border", (205, 208, 218)) if theme else (205, 208, 218)
        self.text_color = theme.get("text", (35, 36, 42)) if theme else (35, 36, 42)

        self.tile_colors = {
            0: (208, 225, 190),
            1: (210, 120, 120),
            2: (230, 215, 155),
            3: (140, 180, 225),
            4: (180, 180, 185),
        }

        self.icon_colors = {
            "SCN": (85, 120, 220),
            "ENT": (0, 122, 255),
            "AST": (80, 150, 135),
            "NEW": (60, 160, 110),
            "PY": (150, 90, 210),
            "PFB": (190, 105, 215),
            "MAP": (90, 160, 95),
            "CMP": (230, 150, 70),
            "UI": (70, 150, 210),
            "VS": (145, 90, 220),
            "RTS": (210, 95, 95),
            "RUN": (70, 170, 120),
            "PLG": (95, 120, 190),
            "LAY": (100, 110, 130),
            "CFG": (80, 95, 130),
            "IMG": (80, 150, 135),
            "AUD": (180, 110, 60),
            "DAT": (90, 110, 160),
            "SYS": (100, 110, 130),
            "FILE": (100, 110, 130),
            "LCK": (120, 120, 125),
            "HID": (150, 150, 150),
        }

        self.asset_icons = {
            "Sprite": "IMG",
            "Audio": "AUD",
            "Data": "DAT",
            "Script": "PY",
            "Component": "CMP",
            "System": "SYS",
            "Prefab": "PFB",
            "Scene": "SCN",
            "Settings": "CFG",
            "Plugin": "CFG",
            "Project": "CFG",
        }

        self.scaled_sprite_cache = {}

    # =========================
    # MAIN DRAW
    # =========================

    def draw(self):
        screen = self.game.screen
        screen.fill(self.bg_color)

        self.safe_draw("world area", self.draw_world_area)
        self.safe_draw("grid chunks", self.draw_grid_chunks)
        self.safe_draw("tilemap layers", self.draw_tilemap_layers)
        self.safe_draw("units", self.draw_units)
        self.safe_draw("game ui", self.draw_game_ui)

        if getattr(self.game, "runtime_mode", False):
            pygame.display.flip()
            return

        self.safe_draw("selection box", self.draw_selection_box)

        if self.game.view_mode.is_scene_view():
            if self.game.editor_view_settings.get("show_gizmos", True):
                self.safe_draw("gizmos", self.draw_gizmos)

            self.safe_draw("navigator", self.draw_left_panel)
            self.safe_draw("navigator scrollbar", self.draw_left_scrollbar)
            self.safe_draw("asset preview", self.draw_content_preview)
            self.safe_draw("project browser", self.draw_project_browser)
            self.safe_draw("scene hierarchy", self.draw_scene_hierarchy)
            self.safe_draw("minimap", self.draw_minimap)
            self.safe_draw("inspector", self.draw_inspector)
            self.safe_draw("console", self.draw_console)
            self.safe_draw("settings panel", self.draw_settings_panel)
            self.safe_draw("drag preview", self.draw_drag_preview)
            self.safe_draw("browser context menu", self.draw_browser_context_menu)
            self.safe_draw("create asset modal", self.draw_create_asset_modal)
            self.safe_draw("visual messages", self.draw_visual_messages)
            

        self.safe_draw("bottom bar", self.draw_bottom_bar)
        self.safe_draw("editor tabs", self.draw_editor_tabs)
        self.safe_draw("script editor", self.draw_script_editor)
        self.safe_draw("command palette", self.draw_command_palette)
        self.safe_draw("visual input editor", self.draw_visual_input_editor)
        self.safe_draw("diagnostics", self.draw_diagnostics_overlay)
        self.safe_draw("autosave banner", self.draw_autosave_recovery_banner)

        if self.game.view_mode.is_scene_view():
            if hasattr(self.game, "toolbar"):
                self.safe_draw("toolbar", self.game.toolbar.draw, screen)

            if hasattr(self.game, "menu_bar"):
                self.safe_draw("menu bar", self.game.menu_bar.draw, screen)

        pygame.display.flip()

    def safe_draw(self, name, callback, *args):
        try:
            callback(*args)
        except Exception as error:
            if hasattr(self.game, "console"):
                self.game.console.log(
                    f"Render panel error [{name}]: {error}",
                    "ERROR"
                )

    def draw_editor_tabs(self):
        if getattr(self.game, "runtime_mode", False):
            return

        if not hasattr(self.game, "editor_tabs"):
            return

        x = 220
        y = 66

        for tab in self.game.editor_tabs.TABS:
            rect = pygame.Rect(x, y, 74, 22)
            active = tab == self.game.editor_tabs.active
            self.draw_button(rect, tab, active=active)
            x += 80

    # =========================
    # BASIC HELPERS
    # =========================

    def draw_text(self, text, x, y, color=None, font=None):
        color = color or self.text_color
        font = font or self.small_font

        img = font.render(str(text), True, color)
        self.game.screen.blit(img, (x, y))
        return img

    def draw_button(self, rect, label, active=False, danger=False):
        mouse = pygame.mouse.get_pos()

        if danger:
            base = (255, 235, 235)
            hover = (255, 220, 220)
            border = (230, 130, 130)
            color = (150, 50, 50)

        elif active:
            base = (0, 122, 255)
            hover = (0, 122, 255)
            border = (0, 122, 255)
            color = (255, 255, 255)

        else:
            base = (245, 247, 252)
            hover = (225, 232, 248)
            border = (205, 208, 218)
            color = (45, 48, 56)

        pygame.draw.rect(
            self.game.screen,
            hover if rect.collidepoint(mouse) else base,
            rect,
            border_radius=7
        )

        pygame.draw.rect(
            self.game.screen,
            border,
            rect,
            1,
            border_radius=7
        )

        img = self.small_font.render(str(label), True, color)

        self.game.screen.blit(
            img,
            (
                rect.x + (rect.width - img.get_width()) // 2,
                rect.y + (rect.height - img.get_height()) // 2,
            )
        )

    def draw_icon_badge(self, rect, label, color):
        pygame.draw.rect(self.game.screen, color, rect, border_radius=5)

        img = self.tiny_font.render(str(label), True, (255, 255, 255))

        self.game.screen.blit(
            img,
            (
                rect.x + (rect.width - img.get_width()) // 2,
                rect.y + (rect.height - img.get_height()) // 2,
            )
        )

    def panel(self, rect, title=None):
        pygame.draw.rect(self.game.screen, self.panel_color, rect, border_radius=10)
        pygame.draw.rect(self.game.screen, self.panel_border, rect, 1, border_radius=10)

        if title:
            self.draw_text(
                title,
                rect.x + 12,
                rect.y + 10,
                self.text_color,
                self.font
            )

    def get_panel_rect(self, panel_id, fallback):
        if hasattr(self.game, "editor_tabs"):
            if not self.game.editor_tabs.panel_visible(panel_id):
                return None

        if hasattr(self.game, "layout_manager"):
            panel = self.game.layout_manager.get(panel_id)

            if panel and panel.visible:
                panel.draw_base(
                    self.game.screen,
                    self.font,
                    self.small_font
                )

                if panel.collapsed:
                    return None

                return panel.content_rect()

        self.panel(fallback, None)
        return fallback

    # =========================
    # WORLD
    # =========================

    def draw_world_area(self):
        screen = self.game.screen
        world_rect = self.game.get_world_viewport_rect()
        self.game.camera.set_viewport(world_rect)

        color = (
            (232, 234, 240)
            if self.game.view_mode.is_scene_view()
            else (20, 22, 28)
        )

        pygame.draw.rect(screen, color, world_rect)
        pygame.draw.rect(screen, (200, 204, 215), world_rect, 1)

        if self.game.mode == "PLAY":
            badge = pygame.Rect(world_rect.x + 12, world_rect.y + 12, 110, 28)
            pygame.draw.rect(screen, (0, 122, 255), badge, border_radius=8)

            self.draw_text(
                "PLAY MODE",
                badge.x + 18,
                badge.y + 8,
                (255, 255, 255),
                self.small_font
            )

        if self.game.view_mode.is_scene_view() and hasattr(self.game, "scene_view_tools"):
            label = (
                f"{self.game.scene_view_tools.gizmo_mode} | "
                f"Snap {'ON' if self.game.scene_view_tools.grid_snapping else 'OFF'}"
            )
            badge = pygame.Rect(world_rect.x + 12, world_rect.y + 46, 150, 24)
            pygame.draw.rect(screen, (252, 252, 254), badge, border_radius=7)
            pygame.draw.rect(screen, self.panel_border, badge, 1, border_radius=7)
            self.draw_text(label, badge.x + 10, badge.y + 6, self.text_color, self.tiny_font)

    def draw_grid_chunks(self):
        grid = self.game.grid
        cam = self.game.camera
        screen_width, screen_height = self.game.screen.get_size()

        show_grid = self.game.editor_view_settings.get("show_grid", True)
        show_chunks = self.game.editor_view_settings.get("show_chunks", False)

        visible_chunks = []

        try:
            if hasattr(grid, "get_visible_chunks"):
                visible_chunks = grid.get_visible_chunks(
                    cam,
                    screen_width,
                    screen_height
                )

        except Exception as error:
            self.game.console.log(
                f"Error obteniendo chunks visibles: {error}",
                "WARNING"
            )

        if visible_chunks and hasattr(grid, "chunks"):
            for chunk_key in visible_chunks:
                tiles = grid.chunks.get(chunk_key, [])

                if show_chunks and self.game.view_mode.is_scene_view():
                    self.draw_chunk_debug(chunk_key)

                for tile_x, tile_y in tiles:
                    self.draw_single_tile(tile_x, tile_y, show_grid)

        else:
            self.draw_grid_fallback(show_grid)

        if self.game.view_mode.is_scene_view():
            self.draw_map_cursor()

    def draw_tilemap_layers(self):
        tilemap = getattr(self.game, "tilemap_layers", None)

        if not tilemap:
            return

        grid = self.game.grid
        cam = self.game.camera
        viewport = self.game.get_world_viewport_rect()

        start_world = cam.screen_to_world(viewport.left, viewport.top)
        end_world = cam.screen_to_world(viewport.right, viewport.bottom)
        start_x = max(0, int(start_world[0] // grid.tile_size) - 1)
        start_y = max(0, int(start_world[1] // grid.tile_size) - 1)
        end_x = min(grid.width, int(end_world[0] // grid.tile_size) + 2)
        end_y = min(grid.height, int(end_world[1] // grid.tile_size) + 2)

        for layer in sorted(tilemap.layers, key=lambda item: item.sorting_order):
            if not layer.visible:
                continue

            alpha = max(0, min(255, int(layer.opacity * 255)))

            for tile_y in range(start_y, end_y):
                for tile_x in range(start_x, end_x):
                    tile_value = layer.get(tile_x, tile_y)

                    if tile_value == layer.default_tile:
                        continue

                    world_x = tile_x * grid.tile_size
                    world_y = tile_y * grid.tile_size
                    screen_x, screen_y = cam.world_to_screen(world_x, world_y)
                    size = int(grid.tile_size * cam.zoom)

                    if size <= 2:
                        continue

                    rect = pygame.Rect(int(screen_x), int(screen_y), size, size)
                    color = self.tile_colors.get(tile_value, (230, 230, 235))
                    overlay = pygame.Surface((rect.width, rect.height), pygame.SRCALPHA)
                    overlay.fill((*color, alpha))
                    self.game.screen.blit(overlay, rect)

                    if layer.collision and self.game.view_mode.is_scene_view():
                        pygame.draw.rect(self.game.screen, (190, 70, 70), rect, 1)

        if self.game.view_mode.is_scene_view():
            active = tilemap.active_layer

            if active:
                badge = pygame.Rect(viewport.right - 190, viewport.y + 12, 176, 24)
                pygame.draw.rect(self.game.screen, (252, 252, 254), badge, border_radius=7)
                pygame.draw.rect(self.game.screen, self.panel_border, badge, 1, border_radius=7)
                lock = "LOCK" if active.locked else "EDIT"
                self.draw_text(
                    f"Layer: {active.name} [{lock}]",
                    badge.x + 8,
                    badge.y + 6,
                    self.text_color,
                    self.tiny_font
                )

    def draw_grid_fallback(self, show_grid=True):
        grid = self.game.grid

        for y in range(grid.height):
            for x in range(grid.width):
                self.draw_single_tile(x, y, show_grid)

    def draw_single_tile(self, tile_x, tile_y, show_grid=True):
        grid = self.game.grid
        cam = self.game.camera

        if not grid.is_inside(tile_x, tile_y):
            return

        world_x = tile_x * grid.tile_size
        world_y = tile_y * grid.tile_size

        screen_x, screen_y = cam.world_to_screen(world_x, world_y)
        size = int(grid.tile_size * cam.zoom)

        if size <= 2:
            return

        rect = pygame.Rect(
            int(screen_x),
            int(screen_y),
            size,
            size
        )

        if rect.right < 0 or rect.bottom < 0:
            return

        if rect.x > self.game.screen.get_width():
            return

        if rect.y > self.game.screen.get_height():
            return

        try:
            tile_value = grid.tiles[tile_y][tile_x]
        except Exception:
            tile_value = 0

        color = self.tile_colors.get(tile_value, (220, 220, 220))

        pygame.draw.rect(self.game.screen, color, rect)

        if show_grid and self.game.view_mode.is_scene_view():
            pygame.draw.rect(self.game.screen, (195, 200, 210), rect, 1)

        if self.game.editor_view_settings.get("show_tile_coordinates", False):
            self.draw_tile_coordinates(tile_x, tile_y, rect)

    def draw_game_ui(self):
        canvas = getattr(self.game, "ui_canvas", None)

        if not canvas:
            return

        for entity, element in canvas.elements():
            rect = canvas.element_rect(element)
            alpha = max(0, min(255, int(getattr(element, "opacity", 1.0) * 255)))
            color = tuple(getattr(element, "color", (245, 247, 252)))
            text_color = tuple(getattr(element, "text_color", (35, 36, 42)))
            border_radius = max(0, int(getattr(element, "border_radius", 7)))
            border_color = tuple(getattr(element, "border_color", (180, 185, 198)))

            if element.element_type == "Image" and element.image_name:
                image = self.game.resources.get_image(element.image_name)

                if image:
                    image = pygame.transform.scale(image, rect.size)
                    image.set_alpha(alpha)
                    self.game.screen.blit(image, rect)
                    continue

            surface = pygame.Surface((rect.width, rect.height), pygame.SRCALPHA)
            pygame.draw.rect(surface, (*color, alpha), surface.get_rect(), border_radius=border_radius)

            if element.element_type in ("Button", "Panel", "ProgressBar"):
                pygame.draw.rect(surface, (*border_color, alpha), surface.get_rect(), 1, border_radius=border_radius)

            if element.element_type == "ProgressBar":
                max_progress = max(0.0001, float(getattr(element, "max_progress", 1.0)))
                progress = max(0.0, min(1.0, float(getattr(element, "progress", 0.0)) / max_progress))
                fill_rect = surface.get_rect().inflate(-4, -4)
                fill_rect.width = int(fill_rect.width * progress)

                if fill_rect.width > 0:
                    pygame.draw.rect(surface, (*text_color, max(40, alpha)), fill_rect, border_radius=max(0, border_radius - 2))

            if getattr(canvas, "hover_entity_id", None) == getattr(entity, "id", None):
                pygame.draw.rect(surface, (0, 122, 255, 90), surface.get_rect(), 2, border_radius=border_radius)

            self.game.screen.blit(surface, rect)

            if element.element_type in ("Label", "Button", "ProgressBar") and element.text:
                if getattr(element, "font_size", 0):
                    font = pygame.font.SysFont(None, max(8, int(element.font_size)))
                else:
                    font = self.font if element.element_type == "Button" else self.small_font

                img = font.render(str(element.text), True, text_color)
                padding = max(0, int(getattr(element, "padding", 8)))
                align = getattr(element, "text_align", "center")

                if align == "left":
                    text_x = rect.x + padding
                elif align == "right":
                    text_x = rect.right - img.get_width() - padding
                else:
                    text_x = rect.x + (rect.width - img.get_width()) // 2

                self.game.screen.blit(
                    img,
                    (
                        max(rect.x + padding, text_x),
                        rect.y + max(6, (rect.height - img.get_height()) // 2),
                    )
                )

            if self.game.view_mode.is_scene_view() and getattr(canvas, "show_bounds", True):
                pygame.draw.rect(self.game.screen, (120, 130, 150), rect, 1, border_radius=border_radius)

    def draw_chunk_debug(self, chunk_key):
        chunk_x, chunk_y = chunk_key
        size = self.game.grid.chunk_size * self.game.grid.tile_size

        world_x = chunk_x * size
        world_y = chunk_y * size

        screen_x, screen_y = self.game.camera.world_to_screen(world_x, world_y)

        rect = pygame.Rect(
            int(screen_x),
            int(screen_y),
            int(size * self.game.camera.zoom),
            int(size * self.game.camera.zoom),
        )

        pygame.draw.rect(self.game.screen, (255, 120, 0), rect, 2)

    def draw_tile_coordinates(self, tile_x, tile_y, rect):
        if rect.width < 26:
            return

        self.draw_text(
            f"{tile_x},{tile_y}",
            rect.x + 2,
            rect.y + 2,
            (80, 80, 90),
            self.tiny_font
        )

    def draw_map_cursor(self):
        if not self.game.editor_view_settings.get("show_mouse_tile", True):
            return

        mouse_x, mouse_y = pygame.mouse.get_pos()

        if self.game.is_mouse_over_ui((mouse_x, mouse_y)):
            return

        try:
            grid_x, grid_y = self.game.screen_to_grid((mouse_x, mouse_y))
        except Exception:
            return

        if not self.game.grid.is_inside(grid_x, grid_y):
            return

        brush_size = max(1, int(getattr(self.game, "brush_size", 1)))

        if not self.game.editor_view_settings.get("show_brush_preview", True):
            return

        tool = getattr(self.game, "active_tool", "Select")

        if tool == "Tile":
            color = (0, 122, 255)
        elif tool == "Obstacle":
            color = (210, 80, 80)
        elif tool == "Erase":
            color = (80, 180, 100)
        elif tool == "Entity":
            color = (160, 90, 220)
        else:
            color = (0, 122, 255)

        for y in range(grid_y - brush_size + 1, grid_y + brush_size):
            for x in range(grid_x - brush_size + 1, grid_x + brush_size):
                if not self.game.grid.is_inside(x, y):
                    continue

                world_x = x * self.game.grid.tile_size
                world_y = y * self.game.grid.tile_size

                screen_x, screen_y = self.game.camera.world_to_screen(world_x, world_y)
                size = int(self.game.grid.tile_size * self.game.camera.zoom)

                rect = pygame.Rect(int(screen_x), int(screen_y), size, size)
                pygame.draw.rect(self.game.screen, color, rect, 2, border_radius=3)

    # =========================
    # UNITS
    # =========================

    def draw_units(self):
        entities = []

        if hasattr(self.game, "world") and hasattr(self.game.world, "entities"):
            entities = self.game.world.entities
        else:
            entities = getattr(self.game, "units", [])

        for unit in sorted(entities, key=self.get_entity_sort_key):
            if not getattr(unit, "enabled", True):
                continue

            if not getattr(unit, "visible", True):
                continue

            layer = getattr(unit, "layer", "Default")

            if hasattr(self.game, "layer_visibility"):
                if not self.game.layer_visibility.is_layer_visible(layer):
                    continue

            rect = self.game.get_unit_screen_rect(unit)

            if not self.is_rect_visible(rect):
                continue

            draw_rect = rect.inflate(-8, -8)

            sprite = self.get_unit_sprite(unit)

            if sprite:
                scaled = self.get_scaled_sprite(sprite, draw_rect.size)
                sprite_renderer = unit.get_component("SpriteRenderer") if hasattr(unit, "get_component") else None

                if sprite_renderer and getattr(sprite_renderer, "tint", (255, 255, 255)) != (255, 255, 255):
                    scaled = scaled.copy()
                    scaled.fill(tuple(sprite_renderer.tint) + (255,), special_flags=pygame.BLEND_RGBA_MULT)

                self.game.screen.blit(scaled, draw_rect)
            else:
                color = self.get_unit_draw_color(unit)
                pygame.draw.rect(self.game.screen, color, draw_rect, border_radius=6)

            self.draw_unit_gameplay_overlays(unit, draw_rect)

            if self.game.view_mode.is_scene_view():
                self.draw_unit_editor_overlays(unit, rect, draw_rect)

    def get_scaled_sprite(self, sprite, size):
        cache_key = (id(sprite), int(size[0]), int(size[1]))

        scaled = self.scaled_sprite_cache.get(cache_key)

        if scaled:
            return scaled

        scaled = pygame.transform.scale(sprite, size)
        self.scaled_sprite_cache[cache_key] = scaled

        if len(self.scaled_sprite_cache) > 512:
            self.scaled_sprite_cache.clear()

        return scaled

    def get_entity_sort_key(self, unit):
        sorting_order = 0

        if hasattr(unit, "get_component"):
            sprite_renderer = unit.get_component("SpriteRenderer")

            if sprite_renderer:
                sorting_order = getattr(sprite_renderer, "sorting_order", 0)

        layer = getattr(unit, "layer", "Default")
        return (str(layer), sorting_order, getattr(unit, "y", 0), getattr(unit, "x", 0))

    def is_rect_visible(self, rect):
        screen_rect = self.game.screen.get_rect()
        return rect.colliderect(screen_rect)

    def get_unit_sprite(self, unit):
        sprite_name = getattr(unit, "sprite_name", None)

        if not sprite_name and hasattr(unit, "get_component"):
            sprite_renderer = unit.get_component("SpriteRenderer")

            if sprite_renderer:
                sprite_name = getattr(sprite_renderer, "sprite_name", None)

        if sprite_name:
            try:
                return self.game.resources.get_image(sprite_name)
            except Exception:
                return None

        return None

    def get_unit_draw_color(self, unit):
        color = unit.get_color() if hasattr(unit, "get_color") else (0, 120, 255)

        command = getattr(unit, "command", None)

        if command == "HOLD":
            color = (240, 180, 60)
        elif command == "PATROL":
            color = (180, 90, 220)
        elif command == "ATTACK_MOVE":
            color = (220, 80, 80)
        elif command == "GATHER":
            color = (60, 160, 90)

        return color

    def draw_unit_editor_overlays(self, unit, rect, draw_rect):
        if getattr(unit, "selected", False):
            pygame.draw.rect(
                self.game.screen,
                (0, 122, 255),
                draw_rect,
                3,
                border_radius=6
            )
            pygame.draw.rect(
                self.game.screen,
                (0, 122, 255),
                rect,
                1,
                border_radius=6
            )

        if getattr(unit, "locked", False):
            self.draw_text(
                "LOCK",
                draw_rect.x,
                draw_rect.bottom + 2,
                (80, 80, 85),
                self.tiny_font
            )

        if self.game.editor_view_settings.get("show_paths", True):
            if getattr(unit, "selected", False) and getattr(unit, "path", []):
                self.draw_unit_path(unit, draw_rect)

        if getattr(unit, "command", None):
            self.draw_unit_command_label(unit, draw_rect)

        if self.game.editor_view_settings.get("show_names", True):
            self.draw_unit_name(unit, draw_rect)

        if self.game.editor_view_settings.get("show_colliders", True):
            self.draw_unit_collider(unit)

        self.draw_scene_view_component_gizmos(unit, draw_rect)

    def draw_unit_gameplay_overlays(self, unit, draw_rect):
        if not hasattr(unit, "get_component"):
            return

        health = unit.get_component("Health")

        if health and getattr(health, "max_health", 0) > 0:
            ratio = max(0.0, min(1.0, float(health.health) / float(health.max_health)))
            bg = pygame.Rect(draw_rect.x, draw_rect.y - 7, draw_rect.width, 4)
            fg = pygame.Rect(bg.x, bg.y, int(bg.width * ratio), bg.height)
            pygame.draw.rect(self.game.screen, (55, 60, 70), bg, border_radius=2)
            pygame.draw.rect(self.game.screen, (70, 190, 95), fg, border_radius=2)

        interaction = unit.get_component("Interaction")

        if interaction and getattr(interaction, "active", False):
            label = str(getattr(interaction, "prompt", "Interact"))
            img = self.tiny_font.render(label, True, (35, 36, 42))
            prompt = pygame.Rect(
                draw_rect.centerx - img.get_width() // 2 - 6,
                draw_rect.y - 24,
                img.get_width() + 12,
                16,
            )
            pygame.draw.rect(self.game.screen, (255, 248, 210), prompt, border_radius=5)
            pygame.draw.rect(self.game.screen, (210, 175, 80), prompt, 1, border_radius=5)
            self.game.screen.blit(img, (prompt.x + 6, prompt.y + 3))

        marker = unit.get_component("ObjectiveMarker")

        if marker and getattr(marker, "visible", True):
            color = tuple(getattr(marker, "color", (255, 210, 90)))
            top = (draw_rect.centerx, draw_rect.y - 22)
            pygame.draw.polygon(
                self.game.screen,
                color,
                [
                    top,
                    (top[0] - 6, top[1] + 10),
                    (top[0] + 6, top[1] + 10),
                ],
            )

    def draw_scene_view_component_gizmos(self, unit, draw_rect):
        if not hasattr(unit, "get_component"):
            return

        tile = max(1, getattr(self.game.grid, "tile_size", 32))
        center = draw_rect.center

        light = unit.get_component("Light2D")

        if light:
            radius = int(float(getattr(light, "radius", 1.0)) * tile * self.game.camera.zoom)
            pygame.draw.circle(self.game.screen, tuple(getattr(light, "color", (255, 240, 200))), center, max(2, radius), 1)

        ai = unit.get_component("AIController")

        if ai:
            radius = int(float(getattr(ai, "detection_radius", 1.0)) * tile * self.game.camera.zoom)
            pygame.draw.circle(self.game.screen, (220, 90, 90), center, max(2, radius), 1)

        interaction = unit.get_component("Interaction")

        if interaction:
            radius = int(float(getattr(interaction, "radius", 1.0)) * tile * self.game.camera.zoom)
            pygame.draw.circle(self.game.screen, (70, 150, 230), center, max(2, radius), 1)

    def draw_unit_name(self, unit, unit_rect):
        name = getattr(unit, "name", "Entity")

        if self.game.editor_view_settings.get("show_entity_ids", False):
            name += f" [{getattr(unit, 'id', '')}]"

        img = self.small_font.render(name, True, (35, 36, 42))

        bg = pygame.Rect(
            unit_rect.centerx - img.get_width() // 2 - 4,
            unit_rect.bottom + 4,
            img.get_width() + 8,
            16
        )

        pygame.draw.rect(self.game.screen, (250, 250, 252), bg, border_radius=5)
        pygame.draw.rect(self.game.screen, (200, 204, 215), bg, 1, border_radius=5)
        self.game.screen.blit(img, (bg.x + 4, bg.y + 2))

    def draw_unit_command_label(self, unit, unit_rect):
        command = getattr(unit, "command", None)

        if not command:
            return

        label = self.small_font.render(str(command), True, (35, 36, 42))

        bg_rect = pygame.Rect(
            unit_rect.centerx - label.get_width() // 2 - 4,
            unit_rect.y - 18,
            label.get_width() + 8,
            16
        )

        pygame.draw.rect(self.game.screen, (250, 250, 252), bg_rect, border_radius=5)
        pygame.draw.rect(self.game.screen, (190, 195, 205), bg_rect, 1, border_radius=5)
        self.game.screen.blit(label, (bg_rect.x + 4, bg_rect.y + 2))

    def draw_unit_path(self, unit, unit_rect):
        grid = self.game.grid
        cam = self.game.camera
        last = unit_rect.center

        for path_x, path_y in unit.path[:30]:
            world_x = path_x * grid.tile_size + grid.tile_size / 2
            world_y = path_y * grid.tile_size + grid.tile_size / 2

            next_screen = cam.world_to_screen(world_x, world_y)

            pygame.draw.line(self.game.screen, (0, 122, 255), last, next_screen, 2)
            pygame.draw.circle(
                self.game.screen,
                (0, 122, 255),
                (int(next_screen[0]), int(next_screen[1])),
                3
            )

            last = next_screen

    def draw_unit_collider(self, unit):
        collider = (
            unit.get_component("Collider2D")
            if hasattr(unit, "get_component")
            else None
        )

        if not collider or not getattr(collider, "enabled", True):
            return

        rect = self.game.get_unit_screen_rect(unit).inflate(-8, -8)

        if getattr(collider, "shape", "rect") == "circle":
            pygame.draw.circle(
                self.game.screen,
                (255, 140, 0),
                rect.center,
                rect.width // 2,
                1
            )
        else:
            pygame.draw.rect(
                self.game.screen,
                (255, 140, 0),
                rect,
                1,
                border_radius=4
            )

    def draw_selection_box(self):
        if self.game.view_mode.is_game_view():
            return

        if not self.game.input_handler.dragging:
            return

        if self.game.active_tool not in ["Select", "Move"]:
            return

        x1, y1 = self.game.input_handler.start_pos
        x2, y2 = self.game.input_handler.current_pos

        rect = pygame.Rect(
            min(x1, x2),
            min(y1, y2),
            abs(x2 - x1),
            abs(y2 - y1)
        )

        if rect.width < 4 or rect.height < 4:
            return

        surface = pygame.Surface((rect.width, rect.height), pygame.SRCALPHA)
        surface.fill((0, 122, 255, 35))

        self.game.screen.blit(surface, (rect.x, rect.y))
        pygame.draw.rect(self.game.screen, (0, 122, 255), rect, 2, border_radius=4)

    # =========================
    # GIZMOS
    # =========================

    def draw_gizmos(self):
        if not self.game.selected_units:
            self.draw_prefab_preview()
            return

        for unit in self.game.selected_units:
            rect = self.game.get_unit_screen_rect(unit)
            center = rect.center

            pygame.draw.circle(
                self.game.screen,
                (0, 122, 255),
                center,
                max(14, rect.width // 2),
                2
            )

            pygame.draw.line(
                self.game.screen,
                (220, 70, 70),
                center,
                (center[0] + 34, center[1]),
                3
            )

            pygame.draw.polygon(
                self.game.screen,
                (220, 70, 70),
                [
                    (center[0] + 42, center[1]),
                    (center[0] + 32, center[1] - 5),
                    (center[0] + 32, center[1] + 5),
                ]
            )

            pygame.draw.line(
                self.game.screen,
                (70, 180, 80),
                center,
                (center[0], center[1] - 34),
                3
            )

            pygame.draw.polygon(
                self.game.screen,
                (70, 180, 80),
                [
                    (center[0], center[1] - 42),
                    (center[0] - 5, center[1] - 32),
                    (center[0] + 5, center[1] - 32),
                ]
            )

        self.draw_prefab_preview()

    def draw_prefab_preview(self):
        if self.game.active_tool != "Entity":
            return

        asset = self.game.file_browser.selected_asset

        if not asset or asset["type"] != "Prefab":
            return

        mouse_x, mouse_y = pygame.mouse.get_pos()

        if self.game.is_mouse_over_ui((mouse_x, mouse_y)):
            return

        try:
            grid_x, grid_y = self.game.screen_to_grid((mouse_x, mouse_y))
        except Exception:
            return

        world_x = grid_x * self.game.grid.tile_size
        world_y = grid_y * self.game.grid.tile_size

        screen_x, screen_y = self.game.camera.world_to_screen(world_x, world_y)
        size = int(self.game.grid.tile_size * self.game.camera.zoom)

        rect = pygame.Rect(int(screen_x), int(screen_y), size, size)
        surface = pygame.Surface((rect.width, rect.height), pygame.SRCALPHA)
        surface.fill((160, 90, 220, 90))

        self.game.screen.blit(surface, rect.topleft)
        pygame.draw.rect(self.game.screen, (160, 90, 220), rect, 2, border_radius=4)

    # =========================
    # NAVIGATOR
    # =========================

    def draw_left_panel(self):
        rect = pygame.Rect(8, 72, 190, 565)

        pygame.draw.rect(self.game.screen, (248, 249, 252), rect, border_radius=14)
        pygame.draw.rect(self.game.screen, (205, 208, 218), rect, 1, border_radius=14)

        self.draw_text("Navigator", rect.x + 14, rect.y + 12, (34, 36, 42), self.font)
        self.draw_text("Project Tools", rect.x + 14, rect.y + 32, (110, 112, 120), self.small_font)

        self.draw_navigator_search(rect)

        old_clip = self.game.screen.get_clip()
        self.game.screen.set_clip(rect)

        actions = self.game.get_navigator_actions()
        scroll = getattr(self.game, "navigator_scroll", 0)

        cursor_y = 86
        mouse_pos = pygame.mouse.get_pos()
        query = self.game.navigator_search_text.strip()

        for section_name, items in actions.items():
            opened = self.game.navigator_sections_open.get(section_name, False)
            force_open = bool(query)

            draw_y = rect.y + cursor_y - scroll

            if rect.y - 40 <= draw_y <= rect.bottom:
                header_rect = pygame.Rect(rect.x + 10, draw_y, rect.width - 20, 26)

                pygame.draw.rect(self.game.screen, (235, 238, 245), header_rect, border_radius=8)

                if header_rect.collidepoint(mouse_pos):
                    pygame.draw.rect(self.game.screen, (225, 232, 248), header_rect, border_radius=8)

                icon = self.game.navigator_icons.get(section_name, "SEC")
                icon_color = self.icon_colors.get(icon, (100, 110, 130))

                self.draw_icon_badge(
                    pygame.Rect(header_rect.x + 6, header_rect.y + 5, 28, 16),
                    icon,
                    icon_color
                )

                arrow = "▼" if opened or force_open else "▶"

                text = self.small_font.render(
                    f"{arrow} {section_name}",
                    True,
                    (45, 48, 56)
                )

                self.game.screen.blit(text, (header_rect.x + 40, header_rect.y + 6))

            cursor_y += 32

            if opened or force_open:
                for label, _ in items:
                    item_y = rect.y + cursor_y - scroll

                    if rect.y - 40 <= item_y <= rect.bottom:
                        item_rect = pygame.Rect(rect.x + 18, item_y, rect.width - 34, 24)

                        if item_rect.collidepoint(mouse_pos):
                            pygame.draw.rect(self.game.screen, (215, 228, 255), item_rect, border_radius=7)

                        dot = pygame.Rect(item_rect.x + 5, item_rect.y + 9, 5, 5)
                        pygame.draw.rect(self.game.screen, (0, 122, 255), dot, border_radius=2)

                        item_text = self.small_font.render(label, True, (65, 68, 78))
                        self.game.screen.blit(item_text, (item_rect.x + 16, item_rect.y + 5))

                    cursor_y += 28

            cursor_y += 4

        self.game.screen.set_clip(old_clip)
        self.game.navigator_max_scroll = max(0, cursor_y - rect.height + 40)

    def draw_navigator_search(self, rect):
        search_rect = pygame.Rect(rect.x + 12, rect.y + 52, rect.width - 24, 24)
        active = getattr(self.game, "navigator_search_active", False)
        text = self.game.navigator_search_text

        pygame.draw.rect(
            self.game.screen,
            (255, 255, 255) if active else (242, 244, 249),
            search_rect,
            border_radius=8
        )

        pygame.draw.rect(
            self.game.screen,
            (0, 122, 255) if active else (205, 208, 218),
            search_rect,
            1,
            border_radius=8
        )

        shown = text if text else "Search tools..."
        color = (35, 36, 42) if text else (135, 138, 148)

        self.draw_text(
            shown,
            search_rect.x + 9,
            search_rect.y + 6,
            color,
            self.small_font
        )

    def draw_left_scrollbar(self):
        rect = pygame.Rect(8, 72, 190, 565)

        max_scroll = max(1, getattr(self.game, "navigator_max_scroll", 1))
        current_scroll = getattr(self.game, "navigator_scroll", 0)

        if max_scroll <= 1:
            return

        track = pygame.Rect(rect.right - 8, rect.y + 82, 4, rect.height - 96)
        pygame.draw.rect(self.game.screen, (225, 228, 235), track, border_radius=3)

        thumb_height = max(
            38,
            int(track.height * (rect.height / (rect.height + max_scroll)))
        )

        thumb_y = track.y + int(
            (track.height - thumb_height) * (current_scroll / max_scroll)
        )

        thumb = pygame.Rect(track.x, thumb_y, track.width, thumb_height)
        pygame.draw.rect(self.game.screen, (150, 156, 170), thumb, border_radius=3)

    # =========================
    # ASSET PREVIEW / FILE EXPLORER
    # =========================

    def draw_content_preview(self):
        asset = self.game.file_browser.selected_asset

        if not asset:
            return

        rect = self.get_panel_rect("asset_preview", pygame.Rect(220, 365, 600, 120))

        if not rect:
            return

        asset_type = asset["type"]

        lines = [
            f"Name: {asset['filename']}",
            f"Type: {asset_type}",
            f"GUID: {asset.get('id', '-')}",
            f"Path: {asset.get('relative_path', asset['path'])}",
            f"Folder: {self.game.file_browser.relative(asset.get('folder', '-'))}",
        ]

        text_x = rect.x + 12

        if asset_type == "Sprite":
            sprite = None

            try:
                sprite = self.game.resources.get_image(asset["name"])
            except Exception:
                sprite = None

            if sprite:
                preview_rect = pygame.Rect(rect.x + 12, rect.y + 10, 70, 70)
                scaled = pygame.transform.scale(sprite, (70, 70))
                self.game.screen.blit(scaled, preview_rect)
                pygame.draw.rect(self.game.screen, self.panel_border, preview_rect, 1)
                text_x = rect.x + 95

        if asset_type == "Prefab":
            lines.append("Prefab listo para colocar con herramienta Entity.")
        elif asset_type == "Scene":
            lines.append("Enter o doble click para abrir escena.")
            lines.extend(self.asset_json_summary(asset.get("path"), ["entities", "scene_name"]))
        elif asset_type in ["Script", "Component", "System"]:
            lines.append("Enter o doble click para abrir en editor interno.")
            lines.append(f"Lines: {self.count_file_lines(asset.get('path'))}")
        elif asset_type == "Audio":
            lines.append("Audio importado al proyecto.")
        elif asset_type == "Data":
            lines.append("Archivo de datos del proyecto.")
            lines.extend(self.asset_json_summary(asset.get("path"), ["version", "created_by"]))

        for i, text in enumerate(lines[:6]):
            self.draw_text(
                text,
                text_x,
                rect.y + 10 + i * 18,
                self.text_color,
                self.small_font
            )

    def count_file_lines(self, path):
        try:
            with open(path, "r", encoding="utf-8") as file:
                return len(file.read().splitlines())
        except Exception:
            return "-"

    def asset_json_summary(self, path, keys):
        try:
            with open(path, "r", encoding="utf-8") as file:
                data = json.load(file)
        except Exception:
            return []

        lines = []

        for key in keys:
            value = data.get(key)

            if isinstance(value, list):
                value = len(value)

            if value is not None:
                lines.append(f"{key}: {value}")

        return lines

    def draw_project_browser(self):
        rect = self.get_panel_rect("content_browser", pygame.Rect(220, 500, 600, 130))

        if not rect:
            return

        fb = self.game.file_browser
        assets = fb.get_visible_assets()

        title = (
            f"Project File Explorer | Filter: {fb.filter_type} | "
            f"Folder: {fb.relative(fb.selected_folder or fb.project_path())}"
        )

        self.draw_text(
            title,
            rect.x + 10,
            rect.y + 8,
            (95, 98, 108),
            self.small_font
        )

        self.draw_browser_quick_buttons(rect)

        folder_width = 170 if fb.tree_view else 0

        if fb.tree_view:
            self.draw_folder_tree(rect, folder_width)

        asset_x = rect.x + 12 + folder_width
        asset_y = rect.y + 56
        asset_w = rect.width - folder_width - 20

        if fb.tree_view:
            pygame.draw.line(
                self.game.screen,
                (210, 214, 224),
                (asset_x - 8, asset_y),
                (asset_x - 8, rect.bottom - 8)
            )

        for i in range(fb.max_visible):
            asset_index = fb.scroll + i

            if asset_index >= len(assets):
                break

            asset = assets[asset_index]
            selected = asset == fb.selected_asset

            row_rect = pygame.Rect(
                asset_x,
                asset_y + i * 20,
                asset_w,
                18
            )

            if selected:
                pygame.draw.rect(self.game.screen, (215, 228, 255), row_rect, border_radius=5)
            elif row_rect.collidepoint(pygame.mouse.get_pos()):
                pygame.draw.rect(self.game.screen, (236, 240, 248), row_rect, border_radius=5)

            icon = self.asset_icons.get(asset["type"], "FILE")
            color = self.icon_colors.get(icon, (100, 110, 130))

            self.draw_icon_badge(
                pygame.Rect(row_rect.x + 4, row_rect.y + 2, 30, 14),
                icon,
                color
            )

            label = f"[{asset['type']}] {asset['filename']}"
            self.draw_text(
                label,
                row_rect.x + 42,
                row_rect.y + 2,
                self.text_color,
                self.small_font
            )

        if fb.dragging_asset:
            self.draw_text(
                "Dragging asset... drop on folder",
                rect.x + 12,
                rect.bottom - 18,
                (160, 95, 180),
                self.tiny_font
            )

    def draw_browser_quick_buttons(self, rect):
        buttons = [
            ("+Script", rect.x + 10),
            ("+Folder", rect.x + 82),
            ("+Scene", rect.x + 158),
            ("+Prefab", rect.x + 230),
            ("Import", rect.x + 310),
            ("Refresh", rect.x + 382),
        ]

        y = rect.y + 30

        for label, x in buttons:
            width = 66 if label != "Refresh" else 72

            self.draw_button(
                pygame.Rect(x, y, width, 22),
                label,
                active=label.startswith("+")
            )

    def draw_folder_tree(self, rect, folder_width):
        fb = self.game.file_browser

        start_x = rect.x + 10
        start_y = rect.y + 56
        row_h = 20

        max_rows = max(1, (rect.height - 66) // row_h)

        for i in range(max_rows):
            index = fb.folder_scroll + i

            if index >= len(fb.folders):
                break

            folder = fb.folders[index]
            selected = folder == fb.selected_folder
            hover = folder == fb.drag_hover_folder

            row = pygame.Rect(
                start_x,
                start_y + i * row_h,
                folder_width - 12,
                18
            )

            if selected:
                pygame.draw.rect(self.game.screen, (225, 235, 255), row, border_radius=5)
            elif hover:
                pygame.draw.rect(self.game.screen, (235, 225, 250), row, border_radius=5)
            elif row.collidepoint(pygame.mouse.get_pos()):
                pygame.draw.rect(self.game.screen, (238, 241, 248), row, border_radius=5)

            rel_folder = fb.relative(folder)
            depth = max(0, rel_folder.count(os.sep))
            visible_name = os.path.basename(folder) or rel_folder
            indent = min(26, depth * 8)

            self.draw_text(
                "▸",
                row.x + 4 + indent,
                row.y + 2,
                (120, 125, 138),
                self.tiny_font
            )

            self.draw_text(
                visible_name,
                row.x + 18 + indent,
                row.y + 2,
                (55, 58, 68),
                self.tiny_font
            )

    def draw_drag_preview(self):
        fb = self.game.file_browser

        if not getattr(fb, "dragging_asset", None):
            return

        mx, my = pygame.mouse.get_pos()
        asset = fb.dragging_asset

        rect = pygame.Rect(mx + 14, my + 14, 170, 26)

        pygame.draw.rect(self.game.screen, (252, 252, 254), rect, border_radius=8)
        pygame.draw.rect(self.game.screen, (160, 150, 190), rect, 1, border_radius=8)

        icon = self.asset_icons.get(asset["type"], "FILE")

        self.draw_icon_badge(
            pygame.Rect(rect.x + 8, rect.y + 6, 30, 14),
            icon,
            (150, 90, 210)
        )

        self.draw_text(
            asset["filename"],
            rect.x + 46,
            rect.y + 7,
            (45, 48, 56),
            self.small_font
        )

    def draw_browser_context_menu(self):
        fb = self.game.file_browser

        if not getattr(fb, "context_menu_open", False):
            return

        x, y = fb.context_menu_pos
        items = fb.context_menu_items

        if not items:
            return

        item_h = 24
        width = 180
        height = len(items) * item_h

        screen_w, screen_h = self.game.screen.get_size()

        if x + width > screen_w:
            x = screen_w - width - 8

        if y + height > screen_h:
            y = screen_h - height - 8

        rect = pygame.Rect(x, y, width, height)

        pygame.draw.rect(self.game.screen, (252, 252, 254), rect, border_radius=9)
        pygame.draw.rect(self.game.screen, (180, 185, 200), rect, 1, border_radius=9)

        mouse = pygame.mouse.get_pos()

        for i, (label, action) in enumerate(items):
            row = pygame.Rect(
                x + 4,
                y + i * item_h + 3,
                width - 8,
                item_h - 6
            )

            if label == "---":
                pygame.draw.line(
                    self.game.screen,
                    (210, 214, 224),
                    (x + 10, row.centery),
                    (x + width - 10, row.centery),
                    1
                )
                continue

            if row.collidepoint(mouse):
                pygame.draw.rect(self.game.screen, (215, 228, 255), row, border_radius=6)

            text = self.small_font.render(label, True, (45, 48, 56))
            self.game.screen.blit(text, (row.x + 8, row.y + 5))

    # =========================
    # CREATE / RENAME MODAL
    # =========================

    def draw_create_asset_modal(self):
        modal = getattr(self.game, "create_asset_modal", None)

        if not modal or not modal.visible:
            return

        screen_w, screen_h = self.game.screen.get_size()

        overlay = pygame.Surface((screen_w, screen_h), pygame.SRCALPHA)
        overlay.fill((0, 0, 0, 80))
        self.game.screen.blit(overlay, (0, 0))

        rect = pygame.Rect(
            screen_w // 2 - 190,
            screen_h // 2 - 90,
            380,
            180
        )

        pygame.draw.rect(self.game.screen, (252, 252, 254), rect, border_radius=16)
        pygame.draw.rect(self.game.screen, (180, 185, 200), rect, 1, border_radius=16)

        self.draw_text(
            modal.title,
            rect.x + 18,
            rect.y + 18,
            (35, 36, 42),
            self.font
        )

        self.draw_text(
            "Name:",
            rect.x + 18,
            rect.y + 60,
            (90, 92, 102),
            self.small_font
        )

        input_rect = pygame.Rect(rect.x + 72, rect.y + 54, rect.width - 95, 30)

        pygame.draw.rect(self.game.screen, (245, 247, 252), input_rect, border_radius=8)
        pygame.draw.rect(self.game.screen, (0, 122, 255), input_rect, 1, border_radius=8)

        shown = modal.buffer if modal.buffer else modal.placeholder
        color = (35, 36, 42) if modal.buffer else (135, 138, 148)

        self.draw_text(
            shown,
            input_rect.x + 10,
            input_rect.y + 8,
            color,
            self.small_font
        )

        hint = "Enter: Confirmar | ESC: Cancelar"

        self.draw_text(
            hint,
            rect.x + 18,
            rect.y + 100,
            (120, 122, 132),
            self.small_font
        )

        create_rect = pygame.Rect(rect.right - 190, rect.bottom - 42, 80, 26)
        cancel_rect = pygame.Rect(rect.right - 100, rect.bottom - 42, 80, 26)

        self.draw_button(create_rect, "Create", active=True)
        self.draw_button(cancel_rect, "Cancel")

    # =========================
    # SCENE HIERARCHY
    # =========================

    def draw_scene_hierarchy(self):
        rect = self.get_panel_rect("hierarchy", pygame.Rect(830, 70, 260, 170))

        if not rect:
            return

        hierarchy = self.game.scene_hierarchy

        if hasattr(hierarchy, "ensure_runtime_fields"):
            hierarchy.ensure_runtime_fields()

        entities = hierarchy.get_entities()
        scroll = hierarchy.scroll

        search_rect = pygame.Rect(rect.x + 10, rect.y + 8, rect.width - 20, 20)

        pygame.draw.rect(self.game.screen, (255, 255, 255), search_rect, border_radius=6)

        pygame.draw.rect(
            self.game.screen,
            (0, 122, 255) if hierarchy.search_active else (205, 208, 218),
            search_rect,
            1,
            border_radius=6
        )

        search_text = hierarchy.search_buffer if hierarchy.search_active else hierarchy.search_text
        shown = search_text if search_text else "Search entities..."
        color = (35, 36, 42) if search_text else (135, 138, 148)

        self.draw_text(
            shown,
            search_rect.x + 8,
            search_rect.y + 4,
            color,
            self.tiny_font
        )

        tag_rect = pygame.Rect(rect.x + 10, rect.y + 31, 72, 18)
        layer_rect = pygame.Rect(rect.x + 88, rect.y + 31, 88, 18)
        reset_rect = pygame.Rect(rect.right - 54, rect.y + 31, 44, 18)

        self.draw_button(tag_rect, f"Tag:{hierarchy.filter_tag}", active=hierarchy.filter_tag != "All")
        self.draw_button(layer_rect, f"Layer:{hierarchy.filter_layer}", active=hierarchy.filter_layer != "All")
        self.draw_button(reset_rect, "Reset")

        row_y = rect.y + 56
        row_height = 20

        for i in range(hierarchy.max_visible):
            index = scroll + i

            if index >= len(entities):
                break

            entity = entities[index]
            selected = entity in self.game.selected_units

            row_rect = pygame.Rect(
                rect.x + 10,
                row_y + i * row_height,
                rect.width - 20,
                18
            )

            if selected:
                pygame.draw.rect(self.game.screen, (215, 228, 255), row_rect, border_radius=5)
            elif row_rect.collidepoint(pygame.mouse.get_pos()):
                pygame.draw.rect(self.game.screen, (238, 241, 248), row_rect, border_radius=5)

            icon = (
                "LCK"
                if getattr(entity, "locked", False)
                else "HID"
                if not getattr(entity, "visible", True)
                else "ENT"
            )

            color = self.icon_colors.get(icon, (0, 122, 255))
            depth = hierarchy.depth_of(entity) if hasattr(hierarchy, "depth_of") else 0
            indent = depth * 14

            self.draw_icon_badge(
                pygame.Rect(row_rect.x + 4 + indent, row_rect.y + 2, 28, 14),
                icon,
                color
            )

            prefab_mark = " PFB" if getattr(entity, "is_prefab_instance", False) else ""
            child_mark = "↳ " if getattr(entity, "parent_id", None) else ""
            label = f"{getattr(entity, 'name', 'Entity')}{prefab_mark} [{getattr(entity, 'id', '-')}]"

            self.draw_text(
                child_mark + label,
                row_rect.x + 38 + indent,
                row_rect.y + 2,
                self.text_color,
                self.small_font
            )

    # =========================
    # MINIMAP
    # =========================

    def draw_minimap(self):
        grid = self.game.grid
        rect = self.get_panel_rect("minimap", pygame.Rect(830, 500, 260, 130))

        if not rect:
            return

        map_rect = pygame.Rect(
            rect.x + 10,
            rect.y + 8,
            max(10, rect.width - 20),
            max(10, rect.height - 18)
        )

        pygame.draw.rect(self.game.screen, (230, 232, 238), map_rect)

        cell_w = map_rect.width / grid.width
        cell_h = map_rect.height / grid.height

        for y in range(grid.height):
            for x in range(grid.width):
                tile = grid.tiles[y][x]
                color = self.tile_colors.get(tile, (220, 220, 220))

                px = map_rect.x + x * cell_w
                py = map_rect.y + y * cell_h

                pygame.draw.rect(
                    self.game.screen,
                    color,
                    (
                        int(px),
                        int(py),
                        max(1, int(cell_w) + 1),
                        max(1, int(cell_h) + 1)
                    )
                )

        for unit in self.game.units:
            if not getattr(unit, "enabled", True) or not getattr(unit, "visible", True):
                continue

            ux = map_rect.x + (unit.x / grid.width) * map_rect.width
            uy = map_rect.y + (unit.y / grid.height) * map_rect.height

            team = unit.get_component("Team") if hasattr(unit, "get_component") else None

            if team:
                color = tuple(team.color)
            elif unit in self.game.selected_units:
                color = (0, 122, 255)
            elif getattr(unit, "tag", "") == "Enemy":
                color = (200, 60, 60)
            elif getattr(unit, "tag", "") == "Resource":
                color = (40, 150, 80)
            else:
                color = (30, 30, 35)

            radius = 3 if unit in self.game.selected_units else 2
            pygame.draw.circle(self.game.screen, color, (int(ux), int(uy)), radius)

        screen_w, screen_h = self.game.screen.get_size()

        left_world, top_world = self.game.camera.screen_to_world(0, 0)
        right_world, bottom_world = self.game.camera.screen_to_world(screen_w, screen_h)

        map_w = grid.width * grid.tile_size
        map_h = grid.height * grid.tile_size

        cam_x = map_rect.x + (left_world / map_w) * map_rect.width
        cam_y = map_rect.y + (top_world / map_h) * map_rect.height
        cam_w = ((right_world - left_world) / map_w) * map_rect.width
        cam_h = ((bottom_world - top_world) / map_h) * map_rect.height

        pygame.draw.rect(
            self.game.screen,
            (0, 122, 255),
            (int(cam_x), int(cam_y), int(cam_w), int(cam_h)),
            1
        )

        pygame.draw.rect(self.game.screen, self.panel_border, map_rect, 1)

    # =========================
    # INSPECTOR
    # =========================

    def draw_inspector(self):
        if not hasattr(self.game, "inspector_editor"):
            return

        editor = self.game.inspector_editor
        editor.field_rects.clear()

        rect = self.get_panel_rect("inspector", pygame.Rect(850, 250, 240, 390))

        if not rect:
            return

        if not self.game.selected_units:
            self.draw_text(
                "No hay entidad seleccionada.",
                rect.x + 18,
                rect.y + 18,
                (120, 122, 130),
                self.small_font
            )
            return

        unit = self.game.selected_units[0]
        y = rect.y + 8

        y = self.draw_inspector_section(
            rect,
            y,
            "Entity",
            [
                ("name", getattr(unit, "name", "Unit")),
                ("active", getattr(unit, "enabled", True)),
                ("enabled", getattr(unit, "enabled", True)),
                ("visible", getattr(unit, "visible", True)),
                ("locked", getattr(unit, "locked", False)),
            ],
            editor
        )

        y = self.draw_inspector_section(
            rect,
            y,
            "Transform",
            [
                ("x", round(getattr(unit, "x", 0), 2)),
                ("y", round(getattr(unit, "y", 0), 2)),
                ("rotation", round(getattr(unit, "rotation", 0), 2)),
                ("scale_x", round(getattr(unit, "scale_x", 1), 2)),
                ("scale_y", round(getattr(unit, "scale_y", 1), 2)),
                ("width", round(getattr(unit, "width", 1), 2)),
                ("height", round(getattr(unit, "height", 1), 2)),
                ("radius", round(getattr(unit, "radius", 0), 2)),
                ("local_x", round(getattr(unit, "local_x", 0), 2)),
                ("local_y", round(getattr(unit, "local_y", 0), 2)),
            ],
            editor
        )

        y = self.draw_inspector_section(
            rect,
            y,
            "Movement",
            [
                ("speed", round(getattr(unit, "speed", 0), 2)),
                ("command", getattr(unit, "command", "IDLE")),
                ("state", getattr(unit, "state", "IDLE")),
            ],
            editor
        )

        y = self.draw_inspector_section(
            rect,
            y,
            "Render",
            [
                ("sprite_name", getattr(unit, "sprite_name", None)),
                ("script", getattr(unit, "script", None)),
                ("tag", getattr(unit, "tag", "Untagged")),
                ("layer", getattr(unit, "layer", "Default")),
            ],
            editor
        )

        y = self.draw_inspector_rts_section(rect, y, unit)
        y = self.draw_inspector_components_section(rect, y, unit)
        y = self.draw_inspector_scripts_section(rect, y, unit)
        y = self.draw_inspector_prefab_section(rect, y, unit)
        self.draw_inspector_debug_section(rect, y, unit)

    def draw_inspector_section_header(self, rect, y, section_name):
        opened = self.game.inspector_sections_open.get(section_name, True)
        header = pygame.Rect(rect.x + 10, y, rect.width - 20, 22)

        pygame.draw.rect(self.game.screen, (235, 238, 245), header, border_radius=7)

        if header.collidepoint(pygame.mouse.get_pos()):
            pygame.draw.rect(self.game.screen, (225, 232, 248), header, border_radius=7)

        arrow = "▼" if opened else "▶"

        self.draw_text(
            f"{arrow} {section_name}",
            header.x + 8,
            header.y + 5,
            (45, 48, 56),
            self.small_font
        )

        return opened, y + 24

    def draw_inspector_section(self, rect, y, section_name, fields, editor):
        opened, y = self.draw_inspector_section_header(rect, y, section_name)

        if not opened:
            return y + 4

        for field, value in fields:
            if y > rect.bottom - 20:
                return y

            label_img = self.small_font.render(str(field), True, (110, 112, 120))
            value_rect = pygame.Rect(rect.x + 90, y - 2, max(70, rect.width - 105), 20)

            if editor.editing and editor.field == field:
                pygame.draw.rect(self.game.screen, (215, 228, 255), value_rect, border_radius=5)
                shown_value = editor.buffer
            else:
                pygame.draw.rect(self.game.screen, (242, 243, 247), value_rect, border_radius=5)
                shown_value = value

            value_img = self.small_font.render(str(shown_value), True, self.text_color)

            self.game.screen.blit(label_img, (rect.x + 18, y))
            self.game.screen.blit(value_img, (value_rect.x + 6, y))

            editor.field_rects[field] = value_rect
            y += 20

        return y + 6

    def draw_inspector_rts_section(self, rect, y, unit):
        opened, y = self.draw_inspector_section_header(rect, y, "RTS")

        if not opened:
            return y + 4

        fields = [
            ("command", getattr(unit, "command", "IDLE")),
            ("state", getattr(unit, "state", "IDLE")),
        ]

        for label, value in fields:
            self.draw_text(label, rect.x + 18, y, (110, 112, 120), self.small_font)
            self.draw_text(value, rect.x + 95, y, self.text_color, self.small_font)
            y += 18

        self.draw_button(pygame.Rect(rect.x + 18, y, 58, 22), "Stop")
        self.draw_button(pygame.Rect(rect.x + 82, y, 58, 22), "Hold")

        return y + 30

    def draw_inspector_components_section(self, rect, y, unit):
        opened, y = self.draw_inspector_section_header(rect, y, "Components")

        if not opened:
            return y + 4

        add_rect = pygame.Rect(rect.x + 18, y, rect.width - 36, 22)
        self.draw_button(add_rect, "Add Selected Component")
        self.game.inspector_editor.field_rects["action:add_selected_component"] = add_rect
        y += 28

        button_width = max(52, (rect.width - 46) // 4)
        actions = [
            ("Copy", "copy_component"),
            ("Paste", "paste_component"),
            ("Reset", "reset_component"),
            ("Playable", "preset_playable"),
        ]

        for index, (label, action) in enumerate(actions):
            button_rect = pygame.Rect(
                rect.x + 18 + index * (button_width + 4),
                y,
                button_width,
                21
            )
            self.draw_button(button_rect, label)
            self.game.inspector_editor.field_rects[f"action:{action}"] = button_rect

        y += 28

        preset_actions = [
            ("TopDown", "preset_topdown"),
            ("Enemy", "preset_enemy"),
            ("NPC", "preset_npc"),
            ("Proj", "preset_projectile"),
        ]

        for index, (label, action) in enumerate(preset_actions):
            button_rect = pygame.Rect(
                rect.x + 18 + index * (button_width + 4),
                y,
                button_width,
                21
            )
            self.draw_button(button_rect, label)
            self.game.inspector_editor.field_rects[f"action:{action}"] = button_rect

        y += 28

        for component in getattr(unit, "components", [])[:12]:
            if y > rect.bottom - 24:
                return y

            self.draw_text(
                f"• {component.component_type}",
                rect.x + 20,
                y,
                (0, 95, 190),
                self.small_font
            )

            enabled = "ON" if getattr(component, "enabled", True) else "OFF"

            self.draw_text(
                enabled,
                rect.right - 42,
                y,
                (90, 120, 90),
                self.tiny_font
            )

            y += 18

            for attr, value in self.get_component_editable_fields(component)[:4]:
                if y > rect.bottom - 20:
                    return y

                field = f"component:{component.component_type}:{attr}"
                label_img = self.tiny_font.render(str(attr), True, (120, 122, 130))
                value_rect = pygame.Rect(rect.x + 105, y - 2, max(70, rect.width - 120), 18)

                if self.game.inspector_editor.editing and self.game.inspector_editor.field == field:
                    pygame.draw.rect(self.game.screen, (215, 228, 255), value_rect, border_radius=5)
                    shown_value = self.game.inspector_editor.buffer
                else:
                    pygame.draw.rect(self.game.screen, (242, 243, 247), value_rect, border_radius=5)
                    shown_value = value

                value_img = self.tiny_font.render(str(shown_value), True, self.text_color)
                self.game.screen.blit(label_img, (rect.x + 34, y))
                self.game.screen.blit(value_img, (value_rect.x + 5, y))

                self.game.inspector_editor.field_rects[field] = value_rect
                y += 17

        return y + 6

    def get_component_editable_fields(self, component):
        fields = []

        for key, value in vars(component).items():
            if key == "component_type":
                continue

            if key.startswith("_"):
                continue

            if isinstance(value, (str, int, float, bool)) or value is None:
                fields.append((key, value))

        return fields

    def draw_inspector_scripts_section(self, rect, y, unit):
        opened, y = self.draw_inspector_section_header(rect, y, "Scripts")

        if not opened:
            return y + 4

        scripts = getattr(unit, "scripts", [])

        if not scripts:
            self.draw_text(
                "No scripts attached",
                rect.x + 20,
                y,
                (120, 122, 130),
                self.small_font
            )
            y += 18

        else:
            for script in scripts[:4]:
                self.draw_text(
                    f"• {getattr(script, 'script_name', 'Script')}",
                    rect.x + 20,
                    y,
                    (150, 90, 210),
                    self.small_font
                )
                y += 18

        return y + 6

    def draw_inspector_prefab_section(self, rect, y, unit):
        opened, y = self.draw_inspector_section_header(rect, y, "Prefab")

        if not opened:
            return y + 4

        source = getattr(unit, "prefab_source", None)

        if source:
            self.draw_text(
                "Prefab Instance",
                rect.x + 20,
                y,
                (150, 90, 210),
                self.small_font
            )
            y += 18

            self.draw_text(
                source[-28:],
                rect.x + 20,
                y,
                (110, 112, 120),
                self.tiny_font
            )
            y += 18

            overrides = self.game.prefab_overrides.diff(unit)
            override_text = f"Overrides: {len(overrides)}"
            self.draw_text(
                override_text,
                rect.x + 20,
                y,
                (190, 105, 215) if overrides else (90, 130, 95),
                self.small_font
            )
            y += 18

            for item in overrides[:3]:
                self.draw_text(
                    item["path"][:28],
                    rect.x + 28,
                    y,
                    (130, 85, 170),
                    self.tiny_font
                )
                y += 15

        else:
            self.draw_text(
                "Not a prefab instance",
                rect.x + 20,
                y,
                (120, 122, 130),
                self.small_font
            )
            y += 18

        x = rect.x + 18
        apply_rect = pygame.Rect(x, y, 58, 22)
        revert_rect = pygame.Rect(x + 64, y, 58, 22)
        self.draw_button(apply_rect, "Apply")
        self.draw_button(revert_rect, "Revert")
        self.game.inspector_editor.field_rects["action:apply_prefab"] = apply_rect
        self.game.inspector_editor.field_rects["action:revert_prefab"] = revert_rect

        return y + 30

    def draw_inspector_debug_section(self, rect, y, unit):
        opened, y = self.draw_inspector_section_header(rect, y, "Debug")

        if not opened:
            return y + 4

        debug_lines = [
            f"ID: {getattr(unit, 'id', 'None')}",
            f"Path nodes: {len(getattr(unit, 'path', []))}",
            f"Components: {len(getattr(unit, 'components', []))}",
            f"Scripts: {len(getattr(unit, 'scripts', []))}",
        ]

        for line in debug_lines:
            if y > rect.bottom - 18:
                return y

            self.draw_text(line, rect.x + 20, y, (90, 92, 104), self.tiny_font)
            y += 16

        return y

    # =========================
    # SETTINGS PANELS
    # =========================

    def draw_settings_panel(self):
        panel_name = getattr(self.game, "active_settings_panel", None)

        if not panel_name:
            return

        screen_w, _ = self.game.screen.get_size()
        rect = pygame.Rect(screen_w - 360, 80, 340, 360)

        pygame.draw.rect(self.game.screen, (252, 252, 254), rect, border_radius=14)
        pygame.draw.rect(self.game.screen, (185, 190, 205), rect, 1, border_radius=14)

        self.draw_text(panel_name, rect.x + 14, rect.y + 14, (35, 36, 42), self.font)

        close_rect = pygame.Rect(rect.right - 32, rect.y + 10, 20, 20)
        self.draw_button(close_rect, "X", danger=True)

        if panel_name == "Build":
            self.draw_build_settings_panel(rect)
        elif panel_name == "BuildProfiles":
            self.draw_generic_settings_rows(rect, self.game.get_build_profile_rows())
        elif panel_name == "Input":
            self.draw_generic_settings_rows(rect, self.game.get_input_settings_rows())
        elif panel_name == "Plugins":
            self.draw_generic_settings_rows(rect, self.game.get_plugin_rows())
        elif panel_name == "Viewport":
            self.draw_viewport_settings_panel(rect)
        elif panel_name == "TagsLayers":
            self.draw_tags_layers_panel(rect)

    def draw_generic_settings_rows(self, rect, rows):
        y = rect.y + 52

        for key, value in rows[:12]:
            row = pygame.Rect(rect.x + 12, y, rect.width - 24, 22)
            pygame.draw.rect(self.game.screen, (245, 247, 252), row, border_radius=6)
            self.draw_text(str(key), row.x + 8, row.y + 5, (90, 92, 102), self.small_font)

            shown = (
                self.game.settings_edit_buffer
                if self.game.settings_editing_key == key
                else value
            )

            self.draw_text(str(shown)[:34], row.x + 130, row.y + 5, self.text_color, self.small_font)

            if self.game.settings_editing_key == key:
                pygame.draw.rect(self.game.screen, (0, 122, 255), row, 1, border_radius=6)

            y += 26

    def draw_build_settings_panel(self, rect):
        rows = self.game.get_build_settings_rows()
        y = rect.y + 52

        for key, value in rows[:11]:
            row = pygame.Rect(rect.x + 12, y, rect.width - 24, 22)

            pygame.draw.rect(self.game.screen, (245, 247, 252), row, border_radius=6)

            self.draw_text(key, row.x + 8, row.y + 5, (90, 92, 102), self.small_font)

            shown = (
                self.game.settings_edit_buffer
                if self.game.settings_editing_key == key
                else value
            )

            self.draw_text(shown, row.x + 170, row.y + 5, self.text_color, self.small_font)

            if self.game.settings_editing_key == key:
                pygame.draw.rect(self.game.screen, (0, 122, 255), row, 1, border_radius=6)

            y += 26

    def draw_viewport_settings_panel(self, rect):
        rows = self.game.get_viewport_settings_rows()
        y = rect.y + 52

        for key, value in rows[:12]:
            row = pygame.Rect(rect.x + 12, y, rect.width - 24, 20)

            pygame.draw.rect(self.game.screen, (245, 247, 252), row, border_radius=6)

            self.draw_text(key, row.x + 8, row.y + 4, (90, 92, 102), self.tiny_font)

            if isinstance(value, bool):
                state_rect = pygame.Rect(row.right - 54, row.y + 3, 42, 14)
                self.draw_button(state_rect, "ON" if value else "OFF", active=value)
            else:
                shown = (
                    self.game.settings_edit_buffer
                    if self.game.settings_editing_key == key
                    else value
                )

                self.draw_text(shown, row.x + 210, row.y + 4, self.text_color, self.tiny_font)

            y += 24

    def draw_tags_layers_panel(self, rect):
        self.draw_button(pygame.Rect(rect.x + 12, rect.y + 52, 92, 24), "Add Tag")
        self.draw_button(pygame.Rect(rect.x + 112, rect.y + 52, 100, 24), "Add Layer")
        self.draw_button(pygame.Rect(rect.x + 12, rect.y + 84, 120, 24), "Cycle Sel Tag")
        self.draw_button(pygame.Rect(rect.x + 140, rect.y + 84, 130, 24), "Cycle Sel Layer")

        self.draw_text("Tags", rect.x + 14, rect.y + 126, (35, 36, 42), self.small_font)

        y = rect.y + 146

        for tag in self.game.tags[:7]:
            self.draw_text(f"• {tag}", rect.x + 22, y, (80, 82, 92), self.small_font)
            y += 18

        self.draw_text("Layers", rect.x + 170, rect.y + 126, (35, 36, 42), self.small_font)

        y = rect.y + 146

        for layer in self.game.layers[:7]:
            visible = self.game.layer_visibility.is_layer_visible(layer)
            locked = self.game.layer_visibility.is_layer_locked(layer)

            state = "V" if visible else "H"
            state += " L" if locked else ""

            self.draw_text(
                f"• {layer} [{state}]",
                rect.x + 178,
                y,
                (80, 82, 92),
                self.small_font
            )
            y += 18

    # =========================
    # BOTTOM / CONSOLE / MESSAGES
    # =========================

    def draw_bottom_bar(self):
        screen = self.game.screen
        width, height = screen.get_size()

        rect = pygame.Rect(0, height - 90, width, 90)

        pygame.draw.rect(screen, (245, 246, 250), rect)
        pygame.draw.line(screen, (200, 204, 215), (0, rect.y), (width, rect.y))

        selected_asset = self.game.file_browser.selected_asset
        asset_text = "None"

        if selected_asset:
            asset_text = f"{selected_asset['type']}:{selected_asset['filename']}"

        scene_name = (
            self.game.scene_manager.current_scene
            if hasattr(self.game, "scene_manager")
            else "None"
        )

        view_mode = (
            self.game.view_mode.mode
            if hasattr(self.game, "view_mode")
            else "SCENE_VIEW"
        )

        project_name = os.path.basename(getattr(self.game, "project_path", "Project"))

        text = (
            f"MiniForge 0.6.0 Beta    "
            f"Project: {project_name}    "
            f"Mode: {self.game.mode}    "
            f"View: {view_mode}    "
            f"Tool: {self.game.active_tool}    "
            f"Brush: {self.game.tile_brush_name()} x{getattr(self.game, 'brush_size', 1)}    "
            f"Scene: {scene_name}    "
            f"Dirty: {'Yes' if getattr(self.game, 'scene_dirty', False) else 'No'}    "
            f"Build: {getattr(self.game.build_profiles, 'active', 'Debug')}    "
            f"Asset: {asset_text}    "
            f"Units: {len(self.game.units)}"
        )

        self.draw_text(text, 220, height - 70, self.text_color, self.font)

        controls = (
            "F5 Play | F6 View | F7 Validate Scene | F8 Manifest | F9 Autosave | F10 Validate Project | "
            "Cmd/Ctrl+F Search | Cmd/Ctrl+R Refresh | Right Click Browser | ` Console | TAB Filter"
        )

        self.draw_text(controls, 220, height - 42, (100, 102, 110), self.small_font)

    def draw_console(self):
        if not self.game.console.visible:
            return

        rect = self.get_panel_rect("console", pygame.Rect(220, 455, 430, 165))

        if not rect:
            return

        level_colors = {
            "INFO": (40, 120, 60),
            "WARNING": (180, 120, 30),
            "ERROR": (200, 50, 50),
            "SCRIPT": (150, 90, 210),
            "ENGINE": (40, 90, 180),
            "RTS": (180, 90, 220),
            "ASSET": (80, 130, 130),
            "SCENE": (80, 100, 190),
            "EDITOR": (90, 90, 150),
        }

        logs = self.game.console.visible_logs()

        for i, entry in enumerate(logs):
            if rect.y + 8 + i * 16 > rect.bottom - 30:
                break

            if isinstance(entry, dict):
                level = entry.get("level", "INFO")
                message = entry.get("message", "")
            else:
                level = "INFO"
                message = str(entry)

            color = level_colors.get(level, (40, 120, 60))

            self.draw_text(
                f"[{level}] {message}",
                rect.x + 12,
                rect.y + 8 + i * 16,
                color,
                self.small_font
            )

        if self.game.console.input_active:
            input_rect = pygame.Rect(rect.x + 10, rect.bottom - 26, rect.width - 20, 20)

            pygame.draw.rect(self.game.screen, (235, 238, 245), input_rect, border_radius=5)
            pygame.draw.rect(self.game.screen, (150, 160, 180), input_rect, 1, border_radius=5)

            text = "> " + self.game.console.command_buffer

            self.draw_text(
                text,
                input_rect.x + 6,
                input_rect.y + 3,
                (30, 30, 35),
                self.small_font
            )
        else:
            hint = f"` input | F1 hide | filter {self.game.console.filter_level}"
            self.draw_text(
                hint,
                rect.right - 120,
                rect.bottom - 20,
                (120, 122, 130),
                self.tiny_font
            )

    def draw_visual_messages(self):
        y = 108

        if getattr(self.game.console, "last_error", None):
            self.draw_message_banner(
                self.game.console.last_error,
                "ERROR",
                pygame.Rect(220, y, 560, 30),
                (255, 230, 230),
                (210, 70, 70)
            )
            y += 34

        if getattr(self.game.console, "last_warning", None):
            self.draw_message_banner(
                self.game.console.last_warning,
                "WARNING",
                pygame.Rect(220, y, 560, 30),
                (255, 246, 220),
                (200, 145, 60)
            )

    def draw_message_banner(self, message, level, rect, bg, border):
        pygame.draw.rect(self.game.screen, bg, rect, border_radius=8)
        pygame.draw.rect(self.game.screen, border, rect, 1, border_radius=8)

        self.draw_text(
            f"{level}: {message}",
            rect.x + 12,
            rect.y + 8,
            (70, 60, 50),
            self.small_font
        )

    def draw_autosave_recovery_banner(self):
        if not getattr(self.game, "autosave_available", False):
            return

        rect = pygame.Rect(220, 70, 590, 34)

        pygame.draw.rect(self.game.screen, (255, 244, 210), rect, border_radius=8)
        pygame.draw.rect(self.game.screen, (230, 190, 80), rect, 1, border_radius=8)

        self.draw_text(
            "Autosave disponible. Presiona F9 para recuperarlo.",
            rect.x + 12,
            rect.y + 9,
            (80, 60, 20),
            self.font
        )

    # =========================
    # SCRIPT EDITOR
    # =========================

    def draw_script_editor(self):
        editor = self.game.script_editor

        if not editor.visible:
            return

        screen = self.game.screen
        width, height = screen.get_size()

        rect = pygame.Rect(230, 80, width - 280, height - 180)

        pygame.draw.rect(screen, (28, 29, 34), rect, border_radius=12)
        pygame.draw.rect(screen, (120, 125, 140), rect, 1, border_radius=12)

        title_bar = pygame.Rect(rect.x, rect.y, rect.width, 36)

        pygame.draw.rect(
            screen,
            (42, 43, 50),
            title_bar,
            border_top_left_radius=12,
            border_top_right_radius=12
        )

        self.draw_text("Python Script Editor", rect.x + 14, rect.y + 10, (245, 245, 245), self.font)

        button_x = rect.right - 252
        for label in ["New", "Save", "Run", "Reload"]:
            button_rect = pygame.Rect(button_x, rect.y + 7, 56, 22)
            pygame.draw.rect(screen, (58, 62, 72), button_rect, border_radius=6)
            pygame.draw.rect(screen, (105, 112, 130), button_rect, 1, border_radius=6)
            self.draw_text(label, button_rect.x + 10, button_rect.y + 5, (235, 238, 245), self.tiny_font)
            button_x += 62

        tab_x = rect.x + 170

        for index, document in enumerate(editor.documents[:6]):
            name = os.path.basename(document.filename)
            dirty = "*" if document.dirty else ""
            tab_rect = pygame.Rect(tab_x, rect.y + 7, 112, 22)
            active = index == editor.active_index
            pygame.draw.rect(screen, (65, 75, 95) if active else (48, 50, 58), tab_rect, border_radius=6)
            self.draw_text((name + dirty)[:15], tab_rect.x + 6, tab_rect.y + 5, (235, 238, 245), self.tiny_font)
            tab_x += 118

        hint = "ESC close | Cmd/Ctrl+S save | Cmd+Enter attach | F6 reload | F7 run | Ctrl+Space autocomplete"

        self.draw_text(
            hint,
            rect.x + 14,
            rect.y + 42,
            (185, 188, 200),
            self.small_font
        )

        code_x = rect.x + 14
        code_y = rect.y + 70
        side_width = 230 if editor.show_symbols or editor.show_errors else 0
        code_width = rect.width - 28 - side_width

        visible_count = 28
        start = editor.scroll
        end = min(len(editor.lines), start + visible_count)

        keywords = [
            "class",
            "def",
            "if",
            "else",
            "elif",
            "for",
            "while",
            "return",
            "from",
            "import",
            "pass",
            "True",
            "False",
            "None",
        ]

        for screen_line, real_line in enumerate(range(start, end)):
            line = editor.lines[real_line]

            self.draw_text(
                str(real_line + 1).rjust(3),
                code_x,
                code_y + screen_line * 18,
                (100, 105, 120),
                self.small_font
            )

            color = (230, 230, 235)
            stripped = line.strip()

            if any(
                stripped.startswith(keyword + " ")
                or stripped.startswith(keyword + ":")
                or stripped == keyword
                for keyword in keywords
            ):
                color = (110, 170, 255)

            if stripped.startswith("#"):
                color = (120, 170, 120)

            if "entity." in line:
                color = (240, 210, 120)

            if "game." in line:
                color = (220, 160, 255)

            error = editor.document.syntax_error

            if error and error["line"] == real_line + 1:
                pygame.draw.rect(
                    screen,
                    (70, 34, 38),
                    pygame.Rect(code_x + 40, code_y + screen_line * 18, code_width - 44, 18)
                )

            img = self.code_font.render(line[:120], True, color)
            screen.blit(img, (code_x + 42, code_y + screen_line * 18))

        cursor_screen_line = editor.cursor_line - editor.scroll

        if 0 <= cursor_screen_line < visible_count:
            cursor_x = code_x + 42 + editor.cursor_col * 8
            cursor_y = code_y + cursor_screen_line * 18
            pygame.draw.rect(screen, (255, 255, 255), (cursor_x, cursor_y, 2, 16))

        if editor.show_help:
            self.draw_script_help_panel(rect)

        if editor.show_errors or editor.show_symbols:
            self.draw_script_side_panel(rect, editor)

    def draw_script_side_panel(self, editor_rect, editor):
        panel = pygame.Rect(editor_rect.right - 245, editor_rect.y + 70, 225, editor_rect.height - 90)
        pygame.draw.rect(self.game.screen, (36, 37, 44), panel, border_radius=8)
        pygame.draw.rect(self.game.screen, (100, 105, 120), panel, 1, border_radius=8)

        y = panel.y + 10
        self.draw_text("Outline", panel.x + 10, y, (245, 245, 245), self.small_font)
        y += 24

        if editor.show_symbols:
            for symbol in editor.document.symbols[:8]:
                label = f"{symbol['type']} {symbol['name']}:{symbol['line']}"
                self.draw_text(label, panel.x + 12, y, (190, 210, 245), self.tiny_font)
                y += 17

        y += 8
        self.draw_text("Errors", panel.x + 10, y, (245, 245, 245), self.small_font)
        y += 24

        error = editor.document.syntax_error

        if error:
            self.draw_text(
                f"Line {error['line']}: {error['message']}",
                panel.x + 12,
                y,
                (255, 145, 145),
                self.tiny_font
            )
        else:
            self.draw_text("No syntax errors", panel.x + 12, y, (140, 220, 160), self.tiny_font)

    def draw_script_help_panel(self, editor_rect):
        help_rect = pygame.Rect(editor_rect.right - 270, editor_rect.y + 70, 250, 255)

        pygame.draw.rect(self.game.screen, (36, 37, 44), help_rect, border_radius=8)
        pygame.draw.rect(self.game.screen, (100, 105, 120), help_rect, 1, border_radius=8)

        self.draw_text(
            "Python API",
            help_rect.x + 10,
            help_rect.y + 10,
            (245, 245, 245),
            self.small_font
        )

        api_lines = [
            "entity.x / entity.y",
            "entity.speed",
            "entity.sprite_name",
            "entity.selected",
            "entity.get_component(type)",
            "entity.game.api.find(name)",
            "entity.game.api.instantiate(prefab,x,y)",
            "entity.game.input_map.get_action(action)",
            "entity.game.console.log(text)",
            "entity.path = []",
            "game.spawn_unit_at_grid(x,y)",
            "game.selected_units",
            "dt = delta time",
            "Cmd+Enter: attach to selected",
            "F2: move_right snippet",
            "F3: console log snippet",
            "F4: toggle errors",
            "Ctrl+Space: autocomplete",
        ]

        for i, text in enumerate(api_lines):
            self.draw_text(
                text,
                help_rect.x + 10,
                help_rect.y + 36 + i * 18,
                (200, 215, 245),
                self.small_font
            )

    def draw_command_palette(self):
        palette = getattr(self.game, "command_palette", None)

        if not palette or not palette.visible:
            return

        screen_w, _ = self.game.screen.get_size()
        rect = pygame.Rect(screen_w // 2 - 230, 86, 460, 300)

        pygame.draw.rect(self.game.screen, (252, 252, 254), rect, border_radius=14)
        pygame.draw.rect(self.game.screen, (145, 155, 180), rect, 1, border_radius=14)

        input_rect = pygame.Rect(rect.x + 14, rect.y + 14, rect.width - 28, 32)
        pygame.draw.rect(self.game.screen, (240, 243, 250), input_rect, border_radius=8)
        pygame.draw.rect(self.game.screen, (0, 122, 255), input_rect, 1, border_radius=8)

        shown = palette.query if palette.query else "Search command..."
        color = self.text_color if palette.query else (130, 134, 145)
        self.draw_text(shown, input_rect.x + 10, input_rect.y + 9, color, self.small_font)

        items = palette.filtered()

        for i, (label, _) in enumerate(items[:10]):
            row = pygame.Rect(rect.x + 14, rect.y + 58 + i * 23, rect.width - 28, 21)

            if i == palette.selected_index:
                pygame.draw.rect(self.game.screen, (215, 228, 255), row, border_radius=6)

            self.draw_text(label, row.x + 8, row.y + 5, self.text_color, self.small_font)

    def draw_diagnostics_overlay(self):
        if not hasattr(self.game, "editor_tabs"):
            return

        if self.game.editor_tabs.active != "Debug":
            return

        rows = self.game.diagnostics.rows()
        rect = pygame.Rect(220, 94, 240, 22 + len(rows) * 18)

        pygame.draw.rect(self.game.screen, (252, 252, 254), rect, border_radius=10)
        pygame.draw.rect(self.game.screen, self.panel_border, rect, 1, border_radius=10)
        self.draw_text("Diagnostics", rect.x + 10, rect.y + 8, self.text_color, self.small_font)

        for i, (key, value) in enumerate(rows):
            y = rect.y + 30 + i * 18
            self.draw_text(str(key), rect.x + 10, y, (90, 94, 108), self.tiny_font)
            self.draw_text(str(value), rect.x + 130, y, self.text_color, self.tiny_font)

        profiler = getattr(self.game, "profiler", None)

        if not profiler:
            return

        profiler_rows = profiler.rows()
        profiler_rect = pygame.Rect(rect.right + 14, rect.y, 320, 22 + len(profiler_rows[:16]) * 18)
        pygame.draw.rect(self.game.screen, (252, 252, 254), profiler_rect, border_radius=10)
        pygame.draw.rect(self.game.screen, self.panel_border, profiler_rect, 1, border_radius=10)
        title = "Profiler" + (" PAUSED" if profiler.paused else "")
        self.draw_text(title, profiler_rect.x + 10, profiler_rect.y + 8, self.text_color, self.small_font)

        for i, (key, value) in enumerate(profiler_rows[:16]):
            y = profiler_rect.y + 30 + i * 18
            self.draw_text(str(key), profiler_rect.x + 10, y, (90, 94, 108), self.tiny_font)
            self.draw_text(str(value), profiler_rect.x + 172, y, self.text_color, self.tiny_font)

    def draw_visual_input_editor(self):
        editor = getattr(self.game, "visual_input_editor", None)

        if not editor or not editor.visible:
            return

        screen_w, screen_h = self.game.screen.get_size()
        rect = pygame.Rect(screen_w // 2 - 330, 92, 660, min(520, screen_h - 140))
        pygame.draw.rect(self.game.screen, (252, 252, 254), rect, border_radius=14)
        pygame.draw.rect(self.game.screen, (150, 158, 180), rect, 1, border_radius=14)

        self.draw_text("Visual Input Editor", rect.x + 16, rect.y + 14, self.text_color, self.font)

        close_rect = pygame.Rect(rect.right - 36, rect.y + 12, 24, 22)
        self.draw_button(close_rect, "X", danger=True)

        actions = editor.actions()
        list_rect = pygame.Rect(rect.x + 16, rect.y + 52, 210, rect.height - 72)
        detail_rect = pygame.Rect(rect.x + 240, rect.y + 52, rect.width - 256, rect.height - 72)

        pygame.draw.rect(self.game.screen, (242, 245, 250), list_rect, border_radius=8)
        pygame.draw.rect(self.game.screen, (242, 245, 250), detail_rect, border_radius=8)

        for i, action in enumerate(actions[editor.scroll:editor.scroll + 16]):
            row = pygame.Rect(list_rect.x + 8, list_rect.y + 8 + i * 24, list_rect.width - 16, 22)
            active = action == editor.selected_action
            self.draw_button(row, action, active=active)

        y = detail_rect.y + 14
        self.draw_text(f"Action: {editor.selected_action}", detail_rect.x + 14, y, self.text_color, self.font)
        y += 36

        bindings = self.game.input_map.bindings.get(editor.selected_action, [])
        self.draw_text("Bindings", detail_rect.x + 14, y, (90, 94, 108), self.small_font)
        y += 24

        for binding in bindings:
            badge = pygame.Rect(detail_rect.x + 14, y, 100, 24)
            self.draw_button(badge, binding)
            y += 30

        y += 8
        capture_rect = pygame.Rect(detail_rect.x + 14, y, 130, 28)
        remove_rect = pygame.Rect(detail_rect.x + 154, y, 130, 28)
        new_rect = pygame.Rect(detail_rect.x + 294, y, 90, 28)
        self.draw_button(capture_rect, "Capture Key", active=editor.capture_mode)
        self.draw_button(remove_rect, "Remove Last")
        self.draw_button(new_rect, "New")

        y += 44
        hint = "Presiona Capture Key y luego una tecla/mouse. Los cambios se guardan en settings/input_map.json."
        self.draw_text(hint, detail_rect.x + 14, y, (90, 94, 108), self.small_font)

        if editor.capture_mode:
            self.draw_text("Capturing...", detail_rect.x + 14, y + 28, (210, 110, 60), self.font)
