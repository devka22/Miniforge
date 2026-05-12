import heapq

try:
    from pathfinding.astar import astar as external_astar
except Exception:
    external_astar = None

from engine.formation import Formation


class CommandSystem:
    """
    RTS Command System 2.

    Soporta:
    - Move
    - Formation Move
    - Stop
    - Hold
    - Patrol
    - Attack Move
    - Follow
    - Guard
    - Gather
    - Cancel

    Incluye A* fallback interno para no depender de imports externos.
    """

    def __init__(self, game):
        self.game = game
        self.default_formation = "square"
        self.formation_spacing = 1

    # =========================
    # PATHFINDING
    # =========================

    def heuristic(self, a, b):
        return abs(a[0] - b[0]) + abs(a[1] - b[1])

    def fallback_astar(self, grid, start, goal):
        if not grid.is_inside(*start):
            return []

        if not grid.is_inside(*goal):
            goal = (
                max(0, min(grid.width - 1, goal[0])),
                max(0, min(grid.height - 1, goal[1]))
            )

        if not grid.is_walkable(*goal):
            goal = grid.nearest_walkable(*goal)

        open_set = []
        heapq.heappush(open_set, (0, start))

        came_from = {}
        cost_so_far = {start: 0}

        while open_set:
            _, current = heapq.heappop(open_set)

            if current == goal:
                break

            neighbors = [
                (current[0] + 1, current[1]),
                (current[0] - 1, current[1]),
                (current[0], current[1] + 1),
                (current[0], current[1] - 1),
            ]

            for next_node in neighbors:
                if not grid.is_inside(*next_node):
                    continue

                if not grid.is_walkable(*next_node):
                    continue

                new_cost = cost_so_far[current] + 1

                if next_node not in cost_so_far or new_cost < cost_so_far[next_node]:
                    cost_so_far[next_node] = new_cost
                    priority = new_cost + self.heuristic(next_node, goal)
                    heapq.heappush(open_set, (priority, next_node))
                    came_from[next_node] = current

        path = []
        current = goal

        while current != start:
            if current not in came_from:
                return []

            path.append(current)
            current = came_from[current]

        path.reverse()
        return path

    def build_path(self, start, goal):
        grid = self.game.grid

        start = (int(start[0]), int(start[1]))
        goal = (int(goal[0]), int(goal[1]))

        if not grid.is_inside(*start):
            return []

        if not grid.is_inside(*goal):
            goal = (
                max(0, min(grid.width - 1, goal[0])),
                max(0, min(grid.height - 1, goal[1]))
            )

        if not grid.is_walkable(*goal):
            goal = grid.nearest_walkable(*goal)

        if external_astar:
            try:
                return external_astar(grid, start, goal)
            except Exception as error:
                self.game.console.log(f"A* externo falló, usando fallback: {error}", "WARNING")

        return self.fallback_astar(grid, start, goal)

    def clean_target(self, target):
        if not target:
            return None

        x, y = int(target[0]), int(target[1])

        if not self.game.grid.is_inside(x, y):
            x = max(0, min(self.game.grid.width - 1, x))
            y = max(0, min(self.game.grid.height - 1, y))

        if not self.game.grid.is_walkable(x, y):
            x, y = self.game.grid.nearest_walkable(x, y)

        return x, y

    # =========================
    # BASIC COMMANDS
    # =========================

    def move_specific_unit_to(self, unit, target):
        target = self.clean_target(target)

        if not target:
            return False

        start = (int(unit.x), int(unit.y))
        unit.path = self.build_path(start, target)
        unit.command = "MOVE"
        unit.state = "MOVING"

        return True

    def move_units_to_specific_unit(self, unit, target):
        return self.move_specific_unit_to(unit, target)

    def move_units(self, target):
        units = list(self.game.selected_units)

        if not units:
            return

        self.formation_move_units(target, self.default_formation)

    def formation_move_units(self, target, formation="square"):
        units = list(self.game.selected_units)

        if not units:
            self.game.console.log("No hay unidades seleccionadas", "WARNING")
            return

        target = self.clean_target(target)

        if not target:
            return

        positions = Formation.create(
            formation,
            target[0],
            target[1],
            len(units),
            self.formation_spacing
        )

        for unit, pos in zip(units, positions):
            clean_pos = self.clean_target(pos)

            if clean_pos:
                self.move_specific_unit_to(unit, clean_pos)
                unit.command = "FORMATION_MOVE"

        self.game.history.take_snapshot("Formation Move")
        self.game.console.log(f"Formation Move: {formation}", "RTS")

    def stop_units(self):
        for unit in self.game.selected_units:
            unit.path = []
            unit.command = "STOP"
            unit.state = "IDLE"
            unit.follow_target_id = None
            unit.guard_target_id = None
            unit.attack_move_target = None
            unit.gather_target_id = None

            worker = unit.get_component("Worker") if hasattr(unit, "get_component") else None

            if worker:
                worker.gather_target_id = None

        self.game.console.log("Unidades detenidas", "RTS")

    def hold_position(self):
        for unit in self.game.selected_units:
            unit.path = []
            unit.command = "HOLD"
            unit.state = "HOLD"

        self.game.console.log("Hold Position", "RTS")

    def cancel_units(self):
        for unit in self.game.selected_units:
            unit.path = []
            unit.command = "IDLE"
            unit.state = "IDLE"
            unit.follow_target_id = None
            unit.guard_target_id = None
            unit.attack_move_target = None
            unit.gather_target_id = None

            worker = unit.get_component("Worker") if hasattr(unit, "get_component") else None

            if worker:
                worker.gather_target_id = None

        self.game.history.take_snapshot("Cancel Command")
        self.game.console.log("Comando cancelado", "RTS")

    # =========================
    # PATROL
    # =========================

    def patrol_units(self, target):
        units = list(self.game.selected_units)

        if not units:
            return

        target = self.clean_target(target)

        if not target:
            return

        for unit in units:
            start = (int(unit.x), int(unit.y))
            unit.patrol_points = [start, target]
            unit.patrol_index = 0
            unit.command = "PATROL"
            self.move_specific_unit_to(unit, target)

        self.game.console.log("Patrol asignado", "RTS")

    # =========================
    # ADVANCED RTS COMMANDS
    # =========================

    def attack_move_units(self, target):
        units = list(self.game.selected_units)

        if not units:
            return

        target = self.clean_target(target)

        if not target:
            return

        for unit in units:
            unit.attack_move_target = target
            unit.command = "ATTACK_MOVE"
            self.move_specific_unit_to(unit, target)

        self.game.history.take_snapshot("Attack Move")
        self.game.console.log("Attack Move asignado", "RTS")

    def follow_units(self, target_entity):
        units = list(self.game.selected_units)

        if not units or not target_entity:
            return

        for unit in units:
            if unit is target_entity:
                continue

            unit.follow_target_id = target_entity.id
            unit.guard_target_id = None
            unit.command = "FOLLOW"
            self.move_specific_unit_to(unit, (int(target_entity.x), int(target_entity.y)))

        self.game.history.take_snapshot("Follow")
        self.game.console.log(f"Follow: {target_entity.name}", "RTS")

    def guard_units(self, target_entity):
        units = list(self.game.selected_units)

        if not units or not target_entity:
            return

        for unit in units:
            if unit is target_entity:
                continue

            unit.guard_target_id = target_entity.id
            unit.follow_target_id = None
            unit.command = "GUARD"
            self.move_specific_unit_to(unit, (int(target_entity.x), int(target_entity.y)))

        self.game.history.take_snapshot("Guard")
        self.game.console.log(f"Guard: {target_entity.name}", "RTS")

    def gather_units(self, target_entity):
        units = list(self.game.selected_units)

        if not units or not target_entity:
            return

        resource = target_entity.get_component("ResourceNode") if hasattr(target_entity, "get_component") else None

        if not resource:
            self.game.console.log("El objetivo no es ResourceNode", "WARNING")
            return

        for unit in units:
            worker = unit.get_component("Worker") if hasattr(unit, "get_component") else None

            if not worker:
                continue

            worker.gather_target_id = target_entity.id
            unit.gather_target_id = target_entity.id
            unit.command = "GATHER"
            self.move_specific_unit_to(unit, (int(target_entity.x), int(target_entity.y)))

        self.game.history.take_snapshot("Gather")
        self.game.console.log(f"Gather: {target_entity.name}", "RTS")

    # =========================
    # TARGET HELPERS
    # =========================

    def find_entity_at_grid(self, grid_x, grid_y):
        for unit in reversed(self.game.units):
            if int(unit.x) == int(grid_x) and int(unit.y) == int(grid_y):
                return unit

        return None

    def command_right_click(self, grid_x, grid_y):
        target_entity = self.find_entity_at_grid(grid_x, grid_y)

        if target_entity:
            if target_entity.get_component("ResourceNode"):
                self.gather_units(target_entity)
                return

            if getattr(target_entity, "tag", "") in ["Player", "Neutral"]:
                self.follow_units(target_entity)
                return

            if getattr(target_entity, "tag", "") == "Enemy":
                self.attack_move_units((grid_x, grid_y))
                return

        self.move_units((grid_x, grid_y))