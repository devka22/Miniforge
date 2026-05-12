import uuid


_entity_name_counters = {}


def generate_entity_id():
    return str(uuid.uuid4())[:8]


def generate_entity_name(prefix="Entity"):
    prefix = str(prefix)

    if prefix not in _entity_name_counters:
        _entity_name_counters[prefix] = 1
    else:
        _entity_name_counters[prefix] += 1

    return f"{prefix}_{_entity_name_counters[prefix]}"


def register_existing_name(name):
    """
    Evita que al cargar escenas se repitan nombres visuales.
    """
    if not name or "_" not in name:
        return

    try:
        prefix, number = name.rsplit("_", 1)
        number = int(number)

        current = _entity_name_counters.get(prefix, 0)

        if number > current:
            _entity_name_counters[prefix] = number

    except Exception:
        pass