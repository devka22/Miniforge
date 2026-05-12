import heapq


def heuristic(a, b):
    """
    Distancia Manhattan para movimiento en grid.
    """
    return abs(a[0] - b[0]) + abs(a[1] - b[1])


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


def astar(grid, start, goal):
    """
    A* optimizado para el motor.
    Usa grid.is_walkable(x, y).
    """

    start = (int(start[0]), int(start[1]))
    goal = (int(goal[0]), int(goal[1]))

    if not grid.is_inside(*start):
        return []

    if not grid.is_inside(*goal):
        return []

    if not grid.is_walkable(*goal):
        goal = grid.nearest_walkable(*goal)

    open_set = []
    heapq.heappush(open_set, (0, start))

    came_from = {}
    cost_so_far = {start: 0}

    closed = set()

    while open_set:
        _, current = heapq.heappop(open_set)

        if current in closed:
            continue

        closed.add(current)

        if current == goal:
            return reconstruct_path(came_from, start, goal)

        neighbors = [
            (current[0] + 1, current[1]),
            (current[0] - 1, current[1]),
            (current[0], current[1] + 1),
            (current[0], current[1] - 1),
        ]

        for next_tile in neighbors:
            if not grid.is_walkable(*next_tile):
                continue

            new_cost = cost_so_far[current] + 1

            if next_tile not in cost_so_far or new_cost < cost_so_far[next_tile]:
                cost_so_far[next_tile] = new_cost

                priority = new_cost + heuristic(next_tile, goal)

                heapq.heappush(open_set, (priority, next_tile))
                came_from[next_tile] = current

    return []