from entities.game_object import GameObject, game_object_from_data
from entities.unit import Unit


Entity = GameObject


def entity_from_json(game, data, preserve_id=False):
    entity_type = data.get("type", "Entity")

    if entity_type in ("Entity", "GameObject", "Player"):
        return game_object_from_data(game, data, preserve_id=preserve_id)

    return Unit(
        data.get("x", data.get("position", [0, 0])[0] if data.get("position") else 0),
        data.get("y", data.get("position", [0, 0])[1] if data.get("position") else 0),
        game,
        entity_id=data.get("id") if preserve_id else None,
        name=data.get("name"),
    )
