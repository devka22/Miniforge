import heapq
import itertools


def heuristic(a, b):
    """
    Heurística Octile, mejor para movimiento diagonal.
    """
    dx = abs(a[0] - b[0])
    dy = abs(a[1] - b[1])

    diagonal = min(dx, dy)
    straight = max(dx, dy) - diagonal

    return 1.4 * diagonal + straight


def get_neighbors(grid, current):
    x, y = current

    directions = [
        (1, 0, 1),
        (-1, 0, 1),
        (0, 1, 1),
        (0, -1, 1),
        (1, 1, 1.4),
        (-1, -1, 1.4),
        (1, -1, 1.4),
        (-1, 1, 1.4),
    ]

    for dx, dy, cost in directions:
        nx = x + dx
        ny = y + dy

        if not grid.is_walkable(nx, ny):
            continue

        # Evitar atravesar esquinas
        if dx != 0 and dy != 0:
            if not grid.is_walkable(x + dx, y):
                continue
            if not grid.is_walkable(x, y + dy):
                continue

        yield (nx, ny), cost


def reconstruct_path(came_from, start, goal):
    path = []
    current = goal

    while current != start:
        if current not in came_from:
            return []

        path.append(current)
        current = came_from[current]

    path.reverse()
    return path


def smooth_path(path):
    """
    Elimina puntos intermedios innecesarios si van en la misma dirección.
    """
    if len(path) < 3:
        return path

    result = [path[0]]

    for i in range(1, len(path) - 1):
        prev = path[i - 1]
        curr = path[i]
        next_point = path[i + 1]

        dir1 = (curr[0] - prev[0], curr[1] - prev[1])
        dir2 = (next_point[0] - curr[0], next_point[1] - curr[1])

        if dir1 != dir2:
            result.append(curr)

    result.append(path[-1])
    return result


def astar(grid, start, goal, max_iterations=3000):
    start = (int(start[0]), int(start[1]))
    goal = (int(goal[0]), int(goal[1]))

    if not grid.is_walkable(*start):
        start = grid.nearest_walkable(*start)

    if not grid.is_walkable(*goal):
        goal = grid.nearest_walkable(*goal)

    open_set = []
    counter = itertools.count()

    heapq.heappush(open_set, (0, next(counter), start))

    came_from = {}
    cost_so_far = {start: 0}

    iterations = 0

    while open_set and iterations < max_iterations:
        iterations += 1

        _, _, current = heapq.heappop(open_set)

        if current == goal:
            path = reconstruct_path(came_from, start, goal)
            return smooth_path(path)

        for neighbor, move_cost in get_neighbors(grid, current):
            new_cost = cost_so_far[current] + move_cost

            if neighbor not in cost_so_far or new_cost < cost_so_far[neighbor]:
                cost_so_far[neighbor] = new_cost
                priority = new_cost + heuristic(neighbor, goal)

                heapq.heappush(open_set, (priority, next(counter), neighbor))
                came_from[neighbor] = current

    return []