import time
import pygame


class SelectionManager:
    """
    Sistema de selección profesional.
    - click select
    - drag select
    - shift add/remove
    - ctrl/cmd + A select all
    - escape clear selection
    - double click same tag
    - ignora locked/layers bloqueadas
    """

    def __init__(self, game):
        self.game = game

        self.last_click_time = 0
        self.last_clicked_entity = None
        self.double_click_time = 0.35

    def can_select(self, entity):
        if not entity:
            return False

        if not getattr(entity, "enabled", True):
            return False

        if not getattr(entity, "visible", True):
            return False

        if getattr(entity, "locked", False):
            return False

        layer = getattr(entity, "layer", "Default")

        if hasattr(self.game, "layer_visibility"):
            if self.game.layer_visibility.is_layer_locked(layer):
                return False

            if not self.game.layer_visibility.is_layer_visible(layer):
                return False

        selectable = None

        if hasattr(entity, "get_component"):
            selectable = entity.get_component("Selectable")

        if selectable and not getattr(selectable, "selectable", True):
            return False

        return True

    def clear(self):
        for unit in self.game.units:
            unit.set_selected(False)

        self.game.selected_units = []

    def add(self, entity):
        if not self.can_select(entity):
            return

        if entity in self.game.selected_units:
            return

        entity.set_selected(True)
        self.game.selected_units.append(entity)

    def remove(self, entity):
        if entity in self.game.selected_units:
            entity.set_selected(False)
            self.game.selected_units.remove(entity)

    def toggle(self, entity):
        if entity in self.game.selected_units:
            self.remove(entity)
        else:
            self.add(entity)

    def select_all(self):
        self.clear()

        for unit in self.game.units:
            self.add(unit)

        self.game.console.log(
            f"Seleccionadas {len(self.game.selected_units)} entidades",
            "EDITOR"
        )

    def select_by_tag(self, tag):
        self.clear()

        for unit in self.game.units:
            if getattr(unit, "tag", "Untagged") == tag:
                self.add(unit)

        self.game.console.log(
            f"Seleccionadas entidades con tag: {tag}",
            "EDITOR"
        )

    def select_at_screen(self, screen_x, screen_y, shift=False):
        clicked = None

        for unit in reversed(self.game.units):
            if not self.can_select(unit):
                continue

            rect = self.game.get_unit_screen_rect(unit)

            collider = unit.get_component("Collider2D") if hasattr(unit, "get_component") else None

            if collider and getattr(collider, "enabled", True):
                if collider.screen_hit_test(unit, self.game, screen_x, screen_y):
                    clicked = unit
                    break

            elif rect.collidepoint(screen_x, screen_y):
                clicked = unit
                break

        if not clicked:
            if not shift:
                self.clear()
            return

        now = time.time()
        is_double_click = (
            self.last_clicked_entity is clicked and
            now - self.last_click_time <= self.double_click_time
        )

        self.last_click_time = now
        self.last_clicked_entity = clicked

        if is_double_click:
            self.select_by_tag(getattr(clicked, "tag", "Untagged"))
            return

        if not shift:
            self.clear()

        if shift:
            self.toggle(clicked)
        else:
            self.add(clicked)

    def select_in_box(self, x1, y1, x2, y2, shift=False, contains=False):
        if not shift:
            self.clear()

        selection_rect = pygame.Rect(
            min(x1, x2),
            min(y1, y2),
            abs(x2 - x1),
            abs(y2 - y1)
        )

        selected_count = 0

        for unit in self.game.units:
            if not self.can_select(unit):
                continue

            unit_rect = self.game.get_unit_screen_rect(unit)

            hit = selection_rect.contains(unit_rect) if contains else selection_rect.colliderect(unit_rect)

            if hit:
                self.add(unit)
                selected_count += 1

        if selected_count:
            self.game.console.log(f"Box selected: {selected_count}", "EDITOR")
