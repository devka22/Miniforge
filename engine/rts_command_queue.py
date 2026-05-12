class RTSCommandQueue:
    """
    Cola simple de comandos RTS.
    Prepara comandos más avanzados sin romper CommandSystem actual.
    """

    def __init__(self, game):
        self.game = game
        self.queue = []

    def push(self, command_type, units, target=None, data=None):
        self.queue.append(
            {
                "type": command_type,
                "units": list(units),
                "target": target,
                "data": data or {},
            }
        )

        self.game.console.log(
            f"Comando en cola: {command_type}",
            "RTS"
        )

    def clear(self):
        self.queue.clear()

    def update(self):
        if not self.queue:
            return

        command = self.queue.pop(0)
        self.execute(command)

    def execute(self, command):
        command_type = command["type"]
        units = command["units"]
        target = command["target"]
        data = command["data"]

        if command_type == "move":
            self.game.command_system.move_units(target)

        elif command_type == "formation_move":
            self.game.command_system.formation_move_units(
                target,
                data.get("formation", "square")
            )

        elif command_type == "attack_move":
            self.game.command_system.attack_move_units(target)

        elif command_type == "follow":
            self.game.command_system.follow_units(target)

        elif command_type == "guard":
            self.game.command_system.guard_units(target)

        elif command_type == "gather":
            self.game.command_system.gather_units(target)

        elif command_type == "cancel":
            self.game.command_system.cancel_units()

        else:
            self.game.console.log(
                f"Comando desconocido: {command_type}",
                "WARNING"
            )