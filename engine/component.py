class Component:
    def __init__(self, component_type):
        self.component_type = component_type
        self.enabled = True

    def start(self, entity):
        pass

    def update(self, entity, dt):
        pass

    def serialize(self):
        return {
            "component_type": self.component_type,
            "enabled": self.enabled,
        }

    def deserialize_base(self, data):
        self.enabled = data.get("enabled", True)


class Transform(Component):
    def __init__(self, x=0, y=0, rotation=0, scale_x=1, scale_y=1):
        super().__init__("Transform")
        self.x = float(x)
        self.y = float(y)
        self.rotation = float(rotation)
        self.scale_x = float(scale_x)
        self.scale_y = float(scale_y)

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "x": self.x,
                "y": self.y,
                "rotation": self.rotation,
                "scale_x": self.scale_x,
                "scale_y": self.scale_y,
            }
        )
        return data


class SpriteRenderer(Component):
    def __init__(self, sprite_name=None, visible=True, sorting_order=0):
        super().__init__("SpriteRenderer")
        self.sprite_name = sprite_name
        self.sprite_guid = None
        self.visible = visible
        self.sorting_order = sorting_order
        self.flip_x = False
        self.flip_y = False
        self.tint = (255, 255, 255)

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "sprite_name": self.sprite_name,
                "sprite_guid": self.sprite_guid,
                "visible": self.visible,
                "sorting_order": self.sorting_order,
                "flip_x": self.flip_x,
                "flip_y": self.flip_y,
                "tint": self.tint,
            }
        )
        return data


class RTSMovement(Component):
    def __init__(self, speed=3.5):
        super().__init__("RTSMovement")
        self.speed = float(speed)
        self.separation = True
        self.allow_pathfinding = True
        self.formation_role = "unit"
        self.acceleration = 1.0
        self.turn_speed = 1.0

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "speed": self.speed,
                "separation": self.separation,
                "allow_pathfinding": self.allow_pathfinding,
                "formation_role": self.formation_role,
                "acceleration": self.acceleration,
                "turn_speed": self.turn_speed,
            }
        )
        return data


class Selectable(Component):
    def __init__(self, selectable=True):
        super().__init__("Selectable")
        self.selectable = selectable
        self.selection_radius = 0.5

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "selectable": self.selectable,
                "selection_radius": self.selection_radius,
            }
        )
        return data


class AudioSource(Component):
    def __init__(self, audio_name=None, volume=1.0, play_on_start=False):
        super().__init__("AudioSource")
        self.audio_name = audio_name
        self.volume = float(volume)
        self.pitch = 1.0
        self.bus = "SFX"
        self.spatial_blend = 0.0
        self.min_distance = 4.0
        self.max_distance = 18.0
        self.play_on_start = play_on_start
        self.loop = False
        self.priority = 128
        self._started = False

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "audio_name": self.audio_name,
                "volume": self.volume,
                "pitch": self.pitch,
                "bus": self.bus,
                "spatial_blend": self.spatial_blend,
                "min_distance": self.min_distance,
                "max_distance": self.max_distance,
                "play_on_start": self.play_on_start,
                "loop": self.loop,
                "priority": self.priority,
            }
        )
        return data


class Rigidbody2D(Component):
    def __init__(self, body_type="dynamic"):
        super().__init__("Rigidbody2D")
        self.body_type = body_type
        self.velocity_x = 0.0
        self.velocity_y = 0.0
        self.angular_velocity = 0.0
        self.mass = 1.0
        self.gravity_scale = 1.0
        self.drag = 0.05
        self.angular_drag = 0.05
        self.bounciness = 0.0
        self.friction = 0.25
        self.use_gravity = True
        self.freeze_x = False
        self.freeze_y = False
        self.freeze_rotation = False
        self.continuous_collision = False
        self.sleeping = False

    @property
    def is_dynamic(self):
        return self.body_type == "dynamic" and not self.sleeping

    @property
    def is_kinematic(self):
        return self.body_type == "kinematic"

    def add_force(self, force_x, force_y, impulse=False):
        mass = max(0.0001, self.mass)
        scale = 1.0 if impulse else 1.0 / mass
        self.velocity_x += float(force_x) * scale
        self.velocity_y += float(force_y) * scale
        self.sleeping = False

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "body_type": self.body_type,
                "velocity_x": self.velocity_x,
                "velocity_y": self.velocity_y,
                "angular_velocity": self.angular_velocity,
                "mass": self.mass,
                "gravity_scale": self.gravity_scale,
                "drag": self.drag,
                "angular_drag": self.angular_drag,
                "bounciness": self.bounciness,
                "friction": self.friction,
                "use_gravity": self.use_gravity,
                "freeze_x": self.freeze_x,
                "freeze_y": self.freeze_y,
                "freeze_rotation": self.freeze_rotation,
                "continuous_collision": self.continuous_collision,
                "sleeping": self.sleeping,
            }
        )
        return data


class Animator(Component):
    def __init__(self):
        super().__init__("Animator")
        self.controller = "Default"
        self.current_state = "Idle"
        self.speed = 1.0
        self.play_on_start = True
        self.loop = True
        self.parameters = {}
        self.preview = True
        self.apply_sprite = True
        self.apply_tint = True
        self.normalized_time = 0.0

    def set_parameter(self, name, value):
        self.parameters[str(name)] = value

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "controller": self.controller,
                "current_state": self.current_state,
                "speed": self.speed,
                "play_on_start": self.play_on_start,
                "loop": self.loop,
                "parameters": self.parameters,
                "preview": self.preview,
                "apply_sprite": self.apply_sprite,
                "apply_tint": self.apply_tint,
                "normalized_time": self.normalized_time,
            }
        )
        return data


class VisualScript(Component):
    def __init__(self):
        super().__init__("VisualScript")
        self.graph_name = "NewGraph"
        self.run_in_editor = False
        self.variables = {}
        self.nodes = [
            {"id": "start", "type": "EventStart", "next": "log"},
            {"id": "log", "type": "Log", "message": "VisualScript started", "next": None},
        ]
        self.enabled_events = ["start", "update", "collision"]
        self._started = False

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "graph_name": self.graph_name,
                "run_in_editor": self.run_in_editor,
                "variables": self.variables,
                "nodes": self.nodes,
                "enabled_events": self.enabled_events,
            }
        )
        return data


class UIElement(Component):
    def __init__(self, element_type="Label"):
        super().__init__("UIElement")
        self.element_type = element_type
        self.text = "Label"
        self.anchor = "top_left"
        self.x = 24.0
        self.y = 24.0
        self.width = 160.0
        self.height = 36.0
        self.color = (245, 247, 252)
        self.text_color = (35, 36, 42)
        self.image_name = None
        self.opacity = 1.0
        self.interactable = element_type == "Button"
        self.on_click_graph = None
        self.sorting_order = 0
        self.padding = 8
        self.border_radius = 7
        self.border_color = (180, 185, 198)
        self.text_align = "center"
        self.font_size = 0
        self.progress = 1.0
        self.max_progress = 1.0

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "element_type": self.element_type,
                "text": self.text,
                "anchor": self.anchor,
                "x": self.x,
                "y": self.y,
                "width": self.width,
                "height": self.height,
                "color": self.color,
                "text_color": self.text_color,
                "image_name": self.image_name,
                "opacity": self.opacity,
                "interactable": self.interactable,
                "on_click_graph": self.on_click_graph,
                "sorting_order": self.sorting_order,
                "padding": self.padding,
                "border_radius": self.border_radius,
                "border_color": self.border_color,
                "text_align": self.text_align,
                "font_size": self.font_size,
                "progress": self.progress,
                "max_progress": self.max_progress,
            }
        )
        return data


class Collider2D(Component):
    def __init__(self, shape="rect", width=1.0, height=1.0, radius=0.5, is_trigger=False):
        super().__init__("Collider2D")
        self.shape = shape
        self.width = float(width)
        self.height = float(height)
        self.radius = float(radius)
        self.is_trigger = is_trigger
        self.offset_x = 0.0
        self.offset_y = 0.0

    def overlaps(self, entity, other):
        dx = (entity.x + self.offset_x) - other.x
        dy = (entity.y + self.offset_y) - other.y

        other_radius = getattr(other, "radius", 0.45)
        radius = self.radius + other_radius

        return (dx * dx + dy * dy) <= radius * radius

    def screen_hit_test(self, entity, game, screen_x, screen_y):
        rect = game.get_unit_screen_rect(entity)

        if self.shape == "circle":
            cx, cy = rect.center
            dx = screen_x - cx
            dy = screen_y - cy
            r = max(rect.width, rect.height) / 2
            return dx * dx + dy * dy <= r * r

        return rect.collidepoint(screen_x, screen_y)

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "shape": self.shape,
                "width": self.width,
                "height": self.height,
                "radius": self.radius,
                "is_trigger": self.is_trigger,
                "offset_x": self.offset_x,
                "offset_y": self.offset_y,
            }
        )
        return data


class Health(Component):
    def __init__(self, max_health=100, armor=0):
        super().__init__("Health")
        self.max_health = float(max_health)
        self.health = float(max_health)
        self.armor = float(armor)
        self.alive = True

    def take_damage(self, amount):
        if not self.alive:
            return

        damage = max(0, float(amount) - self.armor)
        self.health -= damage

        if self.health <= 0:
            self.health = 0
            self.alive = False

    def heal(self, amount):
        if not self.alive:
            return

        self.health = min(self.max_health, self.health + float(amount))

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "max_health": self.max_health,
                "health": self.health,
                "armor": self.armor,
                "alive": self.alive,
            }
        )
        return data


class Team(Component):
    def __init__(self, team_id=0, team_name="Neutral", color=(80, 120, 255)):
        super().__init__("Team")
        self.team_id = int(team_id)
        self.team_name = team_name
        self.color = color

    def is_enemy(self, other_team):
        if not other_team:
            return False

        return self.team_id != other_team.team_id

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "team_id": self.team_id,
                "team_name": self.team_name,
                "color": self.color,
            }
        )
        return data


class ResourceNode(Component):
    def __init__(self, resource_type="Gold", amount=500, gather_rate=10):
        super().__init__("ResourceNode")
        self.resource_type = resource_type
        self.amount = float(amount)
        self.max_amount = float(amount)
        self.gather_rate = float(gather_rate)

    def gather(self, amount):
        gathered = min(self.amount, float(amount))
        self.amount -= gathered
        return gathered

    def is_depleted(self):
        return self.amount <= 0

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "resource_type": self.resource_type,
                "amount": self.amount,
                "max_amount": self.max_amount,
                "gather_rate": self.gather_rate,
            }
        )
        return data


class Worker(Component):
    def __init__(self, carry_capacity=50):
        super().__init__("Worker")
        self.carry_capacity = float(carry_capacity)
        self.carrying_type = None
        self.carrying_amount = 0.0
        self.gather_target_id = None

    def can_carry_more(self):
        return self.carrying_amount < self.carry_capacity

    def add_resource(self, resource_type, amount):
        if self.carrying_type is None:
            self.carrying_type = resource_type

        if self.carrying_type != resource_type:
            return 0

        space = self.carry_capacity - self.carrying_amount
        added = min(space, float(amount))
        self.carrying_amount += added
        return added

    def empty(self):
        data = {
            "type": self.carrying_type,
            "amount": self.carrying_amount,
        }

        self.carrying_type = None
        self.carrying_amount = 0.0

        return data

    def serialize(self):
        data = super().serialize()
        data.update(
            {
                "carry_capacity": self.carry_capacity,
                "carrying_type": self.carrying_type,
                "carrying_amount": self.carrying_amount,
                "gather_target_id": self.gather_target_id,
            }
        )
        return data


class DataComponent(Component):
    fields = ()

    def serialize(self):
        data = super().serialize()

        for field in self.fields:
            data[field] = getattr(self, field)

        return data

    def deserialize_fields(self, data):
        for field in self.fields:
            if field in data:
                setattr(self, field, data[field])


class Stats(DataComponent):
    fields = (
        "level",
        "experience",
        "experience_to_next",
        "strength",
        "agility",
        "intelligence",
        "vitality",
        "attack",
        "defense",
        "magic",
        "resistance",
        "move_speed_bonus",
        "max_health_bonus",
        "critical_chance",
        "critical_multiplier",
        "regen_per_second",
    )

    def __init__(self):
        super().__init__("Stats")
        self.level = 1
        self.experience = 0.0
        self.experience_to_next = 100.0
        self.strength = 5.0
        self.agility = 5.0
        self.intelligence = 5.0
        self.vitality = 5.0
        self.attack = 10.0
        self.defense = 0.0
        self.magic = 0.0
        self.resistance = 0.0
        self.move_speed_bonus = 0.0
        self.max_health_bonus = 0.0
        self.critical_chance = 0.05
        self.critical_multiplier = 1.5
        self.regen_per_second = 0.0

    def add_experience(self, amount):
        self.experience += max(0.0, float(amount))
        levels = 0

        while self.experience >= self.experience_to_next and self.experience_to_next > 0:
            self.experience -= self.experience_to_next
            self.level += 1
            levels += 1
            self.experience_to_next = round(self.experience_to_next * 1.25 + 25.0, 2)

        return levels

    def effective_attack(self):
        return self.attack + self.strength * 0.5

    def effective_defense(self):
        return self.defense + self.vitality * 0.25


class Inventory(DataComponent):
    fields = ("capacity", "items", "currency", "stack_limit", "locked")

    def __init__(self):
        super().__init__("Inventory")
        self.capacity = 24
        self.items = []
        self.currency = {}
        self.stack_limit = 99
        self.locked = False

    def used_slots(self):
        return len(self.items)

    def find_item(self, item_id):
        for item in self.items:
            if item.get("id") == item_id:
                return item
        return None

    def add_item(self, item_id, quantity=1, metadata=None):
        if self.locked:
            return 0

        quantity = max(0, int(quantity))
        added = 0
        item = self.find_item(item_id)

        if item and item.get("stackable", True):
            space = max(0, int(item.get("stack_limit", self.stack_limit)) - int(item.get("quantity", 0)))
            moved = min(space, quantity)
            item["quantity"] = int(item.get("quantity", 0)) + moved
            quantity -= moved
            added += moved

        while quantity > 0 and self.used_slots() < int(self.capacity):
            moved = min(quantity, int(self.stack_limit))
            new_item = {
                "id": str(item_id),
                "quantity": moved,
                "stackable": True,
                "stack_limit": int(self.stack_limit),
                "metadata": metadata or {},
            }
            self.items.append(new_item)
            quantity -= moved
            added += moved

        return added

    def remove_item(self, item_id, quantity=1):
        quantity = max(0, int(quantity))
        removed = 0

        for item in list(self.items):
            if item.get("id") != item_id:
                continue

            available = int(item.get("quantity", 0))
            moved = min(available, quantity)
            item["quantity"] = available - moved
            quantity -= moved
            removed += moved

            if item.get("quantity", 0) <= 0:
                self.items.remove(item)

            if quantity <= 0:
                break

        return removed

    def count_item(self, item_id):
        return sum(int(item.get("quantity", 0)) for item in self.items if item.get("id") == item_id)

    def has_item(self, item_id, quantity=1):
        return self.count_item(item_id) >= int(quantity)

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.items = list(self.items or [])
        self.currency = dict(self.currency or {})


class Equipment(DataComponent):
    fields = ("slots", "stat_bonuses", "locked_slots")

    def __init__(self):
        super().__init__("Equipment")
        self.slots = {
            "weapon": None,
            "armor": None,
            "trinket": None,
            "tool": None,
        }
        self.stat_bonuses = {}
        self.locked_slots = []

    def equip(self, slot, item_id, bonuses=None):
        if slot in self.locked_slots:
            return False

        self.slots[str(slot)] = str(item_id) if item_id is not None else None
        self.stat_bonuses[str(slot)] = bonuses or {}
        return True

    def unequip(self, slot):
        item = self.slots.get(slot)
        self.slots[slot] = None
        self.stat_bonuses.pop(slot, None)
        return item

    def total_bonus(self, stat):
        return sum(float(bonuses.get(stat, 0.0)) for bonuses in self.stat_bonuses.values())

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.slots = dict(self.slots or {})
        self.stat_bonuses = dict(self.stat_bonuses or {})
        self.locked_slots = list(self.locked_slots or [])


class Ability(DataComponent):
    fields = (
        "ability_id",
        "display_name",
        "cooldown",
        "mana_cost",
        "range",
        "power",
        "target_mode",
        "charges",
        "current_charges",
        "recharge_time",
        "last_cast_time",
        "unlocked",
        "tags",
    )

    def __init__(self):
        super().__init__("Ability")
        self.ability_id = "ability"
        self.display_name = "Ability"
        self.cooldown = 1.0
        self.mana_cost = 0.0
        self.range = 4.0
        self.power = 10.0
        self.target_mode = "entity"
        self.charges = 1
        self.current_charges = 1
        self.recharge_time = 0.0
        self.last_cast_time = -9999.0
        self.unlocked = True
        self.tags = []

    def is_ready(self, now):
        return self.unlocked and self.current_charges > 0 and (float(now) - self.last_cast_time) >= self.cooldown

    def trigger(self, now):
        if not self.is_ready(now):
            return False

        self.current_charges = max(0, int(self.current_charges) - 1)
        self.last_cast_time = float(now)
        return True

    def recharge(self, amount=1):
        self.current_charges = min(int(self.charges), int(self.current_charges) + int(amount))

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.tags = list(self.tags or [])


class AIController(DataComponent):
    fields = (
        "behavior",
        "target_id",
        "home_x",
        "home_y",
        "think_interval",
        "think_timer",
        "detection_radius",
        "attack_radius",
        "leash_radius",
        "patrol_radius",
        "wander_radius",
        "target_tags",
        "state",
    )

    def __init__(self):
        super().__init__("AIController")
        self.behavior = "idle"
        self.target_id = None
        self.home_x = 0.0
        self.home_y = 0.0
        self.think_interval = 0.25
        self.think_timer = 0.0
        self.detection_radius = 6.0
        self.attack_radius = 1.25
        self.leash_radius = 12.0
        self.patrol_radius = 4.0
        self.wander_radius = 5.0
        self.target_tags = ["Enemy", "Player"]
        self.state = "idle"

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.target_tags = list(self.target_tags or [])


class NavAgent(DataComponent):
    fields = (
        "has_destination",
        "destination_x",
        "destination_y",
        "speed",
        "stopping_distance",
        "repath_interval",
        "repath_timer",
        "auto_repath",
        "avoid_obstacles",
        "path_smoothing",
        "last_path_length",
    )

    def __init__(self):
        super().__init__("NavAgent")
        self.has_destination = False
        self.destination_x = 0.0
        self.destination_y = 0.0
        self.speed = 3.5
        self.stopping_distance = 0.15
        self.repath_interval = 0.25
        self.repath_timer = 0.0
        self.auto_repath = True
        self.avoid_obstacles = True
        self.path_smoothing = True
        self.last_path_length = 0

    def set_destination(self, x, y):
        self.destination_x = float(x)
        self.destination_y = float(y)
        self.has_destination = True
        self.repath_timer = 9999.0

    def clear_destination(self):
        self.has_destination = False
        self.last_path_length = 0


class Interaction(DataComponent):
    fields = (
        "prompt",
        "radius",
        "action_name",
        "action_graph",
        "requires_tag",
        "single_use",
        "used",
        "active",
    )

    def __init__(self):
        super().__init__("Interaction")
        self.prompt = "Interact"
        self.radius = 1.25
        self.action_name = "interact"
        self.action_graph = None
        self.requires_tag = "Player"
        self.single_use = False
        self.used = False
        self.active = False


class Lifetime(DataComponent):
    fields = ("duration", "elapsed", "destroy_on_expire", "fade_out")

    def __init__(self):
        super().__init__("Lifetime")
        self.duration = 5.0
        self.elapsed = 0.0
        self.destroy_on_expire = True
        self.fade_out = False


class Spawner(DataComponent):
    fields = (
        "prefab_name",
        "spawn_interval",
        "spawn_radius",
        "max_alive",
        "spawn_on_start",
        "enabled_in_editor",
        "spawned_ids",
        "elapsed",
        "started",
    )

    def __init__(self):
        super().__init__("Spawner")
        self.prefab_name = ""
        self.spawn_interval = 5.0
        self.spawn_radius = 2.0
        self.max_alive = 3
        self.spawn_on_start = False
        self.enabled_in_editor = False
        self.spawned_ids = []
        self.elapsed = 0.0
        self.started = False

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.spawned_ids = list(self.spawned_ids or [])


class DamageDealer(DataComponent):
    fields = (
        "damage",
        "damage_type",
        "cooldown",
        "knockback",
        "target_tags",
        "hit_once",
        "last_hits",
    )

    def __init__(self):
        super().__init__("DamageDealer")
        self.damage = 10.0
        self.damage_type = "physical"
        self.cooldown = 0.5
        self.knockback = 0.0
        self.target_tags = ["Enemy"]
        self.hit_once = False
        self.last_hits = {}

    def can_hit(self, entity_id, now):
        if self.hit_once and entity_id in self.last_hits:
            return False

        last = float(self.last_hits.get(entity_id, -9999.0))
        return (float(now) - last) >= float(self.cooldown)

    def mark_hit(self, entity_id, now):
        self.last_hits[str(entity_id)] = float(now)

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.target_tags = list(self.target_tags or [])
        self.last_hits = dict(self.last_hits or {})


class CameraFollow(DataComponent):
    fields = (
        "target_id",
        "smoothness",
        "offset_x",
        "offset_y",
        "zoom",
        "dead_zone",
        "follow_x",
        "follow_y",
    )

    def __init__(self):
        super().__init__("CameraFollow")
        self.target_id = None
        self.smoothness = 8.0
        self.offset_x = 0.0
        self.offset_y = 0.0
        self.zoom = 1.0
        self.dead_zone = 0.0
        self.follow_x = True
        self.follow_y = True


class Saveable(DataComponent):
    fields = ("save_key", "include_components", "persistent", "version", "autosave")

    def __init__(self):
        super().__init__("Saveable")
        self.save_key = ""
        self.include_components = True
        self.persistent = True
        self.version = 1
        self.autosave = True


class Blackboard(DataComponent):
    fields = ("values",)

    def __init__(self):
        super().__init__("Blackboard")
        self.values = {}

    def get(self, key, default=None):
        return self.values.get(str(key), default)

    def set(self, key, value):
        self.values[str(key)] = value

    def increment(self, key, amount=1):
        self.values[str(key)] = self.values.get(str(key), 0) + amount
        return self.values[str(key)]

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.values = dict(self.values or {})


class StateMachine(DataComponent):
    fields = ("current_state", "initial_state", "states", "transitions", "time_in_state", "auto_start")

    def __init__(self):
        super().__init__("StateMachine")
        self.current_state = "Idle"
        self.initial_state = "Idle"
        self.states = ["Idle"]
        self.transitions = []
        self.time_in_state = 0.0
        self.auto_start = True

    def set_state(self, state):
        if state != self.current_state:
            self.current_state = str(state)
            self.time_in_state = 0.0

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.states = list(self.states or [])
        self.transitions = list(self.transitions or [])


class QuestLog(DataComponent):
    fields = ("quests", "active_quest_id", "completed_count")

    def __init__(self):
        super().__init__("QuestLog")
        self.quests = []
        self.active_quest_id = None
        self.completed_count = 0

    def add_quest(self, quest_id, title, objectives=None):
        if any(quest.get("id") == quest_id for quest in self.quests):
            return False

        self.quests.append(
            {
                "id": str(quest_id),
                "title": str(title),
                "state": "active",
                "objectives": objectives or [],
            }
        )
        self.active_quest_id = str(quest_id)
        return True

    def set_objective_progress(self, quest_id, objective_id, value):
        for quest in self.quests:
            if quest.get("id") != quest_id:
                continue

            for objective in quest.get("objectives", []):
                if objective.get("id") == objective_id:
                    objective["progress"] = value
                    return True
        return False

    def complete_quest(self, quest_id):
        for quest in self.quests:
            if quest.get("id") == quest_id:
                quest["state"] = "completed"
                self.completed_count += 1
                return True
        return False

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.quests = list(self.quests or [])


class Dialogue(DataComponent):
    fields = ("speaker", "lines", "index", "is_active", "auto_advance", "choices", "on_complete_graph")

    def __init__(self):
        super().__init__("Dialogue")
        self.speaker = "NPC"
        self.lines = ["Hello."]
        self.index = 0
        self.is_active = False
        self.auto_advance = False
        self.choices = []
        self.on_complete_graph = None

    def current_line(self):
        if not self.lines:
            return ""
        return self.lines[max(0, min(int(self.index), len(self.lines) - 1))]

    def advance(self):
        self.index += 1

        if self.index >= len(self.lines):
            self.is_active = False
            self.index = max(0, len(self.lines) - 1)
            return False

        return True

    def reset(self):
        self.index = 0
        self.is_active = True

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.lines = list(self.lines or [])
        self.choices = list(self.choices or [])


class Cooldown(DataComponent):
    fields = ("timers",)

    def __init__(self):
        super().__init__("Cooldown")
        self.timers = {}

    def start(self, name, duration=None):
        if duration is None:
            return

        self.timers[str(name)] = max(0.0, float(duration))

    def tick(self, dt):
        for key in list(self.timers.keys()):
            self.timers[key] = max(0.0, float(self.timers[key]) - dt)

            if self.timers[key] <= 0.0:
                self.timers.pop(key, None)

    def ready(self, name):
        return str(name) not in self.timers

    def remaining(self, name):
        return float(self.timers.get(str(name), 0.0))

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.timers = dict(self.timers or {})


class StatusEffects(DataComponent):
    fields = ("effects",)

    def __init__(self):
        super().__init__("StatusEffects")
        self.effects = []

    def add_effect(self, name, duration, stacks=1, data=None):
        self.effects.append(
            {
                "name": str(name),
                "duration": float(duration),
                "elapsed": 0.0,
                "stacks": int(stacks),
                "data": data or {},
            }
        )

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.effects = list(self.effects or [])


class CombatTarget(DataComponent):
    fields = (
        "target_id",
        "aggro_radius",
        "attack_radius",
        "lose_radius",
        "target_tags",
        "require_line_of_sight",
    )

    def __init__(self):
        super().__init__("CombatTarget")
        self.target_id = None
        self.aggro_radius = 6.0
        self.attack_radius = 1.25
        self.lose_radius = 10.0
        self.target_tags = ["Enemy"]
        self.require_line_of_sight = False

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.target_tags = list(self.target_tags or [])


class LootTable(DataComponent):
    fields = ("entries", "rolls", "drop_radius", "guaranteed_currency")

    def __init__(self):
        super().__init__("LootTable")
        self.entries = [
            {"id": "coin", "weight": 1.0, "min": 1, "max": 3},
        ]
        self.rolls = 1
        self.drop_radius = 0.5
        self.guaranteed_currency = {}

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.entries = list(self.entries or [])
        self.guaranteed_currency = dict(self.guaranteed_currency or {})


class CameraShake(DataComponent):
    fields = ("amplitude", "duration", "frequency", "elapsed", "trauma", "active")

    def __init__(self):
        super().__init__("CameraShake")
        self.amplitude = 6.0
        self.duration = 0.25
        self.frequency = 24.0
        self.elapsed = 0.0
        self.trauma = 0.0
        self.active = False

    def shake(self, trauma=1.0):
        self.trauma = max(self.trauma, float(trauma))
        self.elapsed = 0.0
        self.active = True


class Light2D(DataComponent):
    fields = ("color", "radius", "intensity", "flicker", "flicker_speed", "casts_shadows")

    def __init__(self):
        super().__init__("Light2D")
        self.color = (255, 240, 200)
        self.radius = 5.0
        self.intensity = 1.0
        self.flicker = False
        self.flicker_speed = 6.0
        self.casts_shadows = False

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.color = tuple(self.color)


class ParallaxLayer(DataComponent):
    fields = ("factor_x", "factor_y", "offset_x", "offset_y", "repeat_x", "repeat_y", "sorting_order")

    def __init__(self):
        super().__init__("ParallaxLayer")
        self.factor_x = 0.5
        self.factor_y = 0.5
        self.offset_x = 0.0
        self.offset_y = 0.0
        self.repeat_x = True
        self.repeat_y = False
        self.sorting_order = -10


class TilemapCollider(DataComponent):
    fields = ("solid_tiles", "one_way_tiles", "friction", "bounciness", "enabled_layers")

    def __init__(self):
        super().__init__("TilemapCollider")
        self.solid_tiles = [1, 3, 4]
        self.one_way_tiles = []
        self.friction = 0.4
        self.bounciness = 0.0
        self.enabled_layers = ["Ground"]

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.solid_tiles = list(self.solid_tiles or [])
        self.one_way_tiles = list(self.one_way_tiles or [])
        self.enabled_layers = list(self.enabled_layers or [])


class ObjectiveMarker(DataComponent):
    fields = ("label", "color", "visible", "max_distance", "pulse", "target_id")

    def __init__(self):
        super().__init__("ObjectiveMarker")
        self.label = "Objective"
        self.color = (255, 210, 90)
        self.visible = True
        self.max_distance = 9999.0
        self.pulse = True
        self.target_id = None

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.color = tuple(self.color)


class Checkpoint(DataComponent):
    fields = ("checkpoint_id", "respawn_x", "respawn_y", "active", "single_use", "activated_by_tag")

    def __init__(self):
        super().__init__("Checkpoint")
        self.checkpoint_id = "checkpoint"
        self.respawn_x = 0.0
        self.respawn_y = 0.0
        self.active = False
        self.single_use = False
        self.activated_by_tag = "Player"


class CharacterController2D(DataComponent):
    fields = (
        "walk_speed",
        "run_speed",
        "jump_force",
        "grounded",
        "coyote_time",
        "coyote_timer",
        "air_control",
        "max_jumps",
        "jumps_used",
        "input_enabled",
    )

    def __init__(self):
        super().__init__("CharacterController2D")
        self.walk_speed = 5.0
        self.run_speed = 7.0
        self.jump_force = 9.0
        self.grounded = False
        self.coyote_time = 0.12
        self.coyote_timer = 0.0
        self.air_control = 0.6
        self.max_jumps = 1
        self.jumps_used = 0
        self.input_enabled = True


class EconomyWallet(DataComponent):
    fields = ("resources", "capacity", "allow_negative")

    def __init__(self):
        super().__init__("EconomyWallet")
        self.resources = {"Gold": 0}
        self.capacity = 999999.0
        self.allow_negative = False

    def add(self, resource_type, amount):
        key = str(resource_type)
        current = float(self.resources.get(key, 0.0))
        self.resources[key] = min(float(self.capacity), current + float(amount))
        return self.resources[key]

    def spend(self, resource_type, amount):
        key = str(resource_type)
        current = float(self.resources.get(key, 0.0))

        if not self.allow_negative and current < float(amount):
            return False

        self.resources[key] = current - float(amount)
        return True

    def deserialize_fields(self, data):
        super().deserialize_fields(data)
        self.resources = dict(self.resources or {})


class Timer(DataComponent):
    fields = ("name", "duration", "elapsed", "loop", "running", "completed")

    def __init__(self):
        super().__init__("Timer")
        self.name = "Timer"
        self.duration = 1.0
        self.elapsed = 0.0
        self.loop = False
        self.running = True
        self.completed = False

    def tick(self, dt):
        if not self.running or self.completed:
            return False

        self.elapsed += max(0.0, float(dt))

        if self.elapsed < self.duration:
            return False

        if self.loop:
            self.elapsed = 0.0
        else:
            self.completed = True
            self.running = False

        return True


class Tween(DataComponent):
    fields = (
        "property_path",
        "from_value",
        "to_value",
        "duration",
        "elapsed",
        "easing",
        "loop",
        "ping_pong",
        "active",
    )

    def __init__(self):
        super().__init__("Tween")
        self.property_path = "x"
        self.from_value = 0.0
        self.to_value = 1.0
        self.duration = 1.0
        self.elapsed = 0.0
        self.easing = "linear"
        self.loop = False
        self.ping_pong = False
        self.active = False

    def sample(self):
        if self.duration <= 0:
            return self.to_value

        t = max(0.0, min(1.0, self.elapsed / self.duration))

        if self.easing == "smooth":
            t = t * t * (3.0 - 2.0 * t)
        elif self.easing == "ease_in":
            t = t * t
        elif self.easing == "ease_out":
            t = 1.0 - (1.0 - t) * (1.0 - t)

        return float(self.from_value) + (float(self.to_value) - float(self.from_value)) * t


ADVANCED_COMPONENT_TYPES = {
    "Stats": Stats,
    "Inventory": Inventory,
    "Equipment": Equipment,
    "Ability": Ability,
    "AIController": AIController,
    "NavAgent": NavAgent,
    "Interaction": Interaction,
    "Lifetime": Lifetime,
    "Spawner": Spawner,
    "DamageDealer": DamageDealer,
    "CameraFollow": CameraFollow,
    "Saveable": Saveable,
    "Blackboard": Blackboard,
    "StateMachine": StateMachine,
    "QuestLog": QuestLog,
    "Dialogue": Dialogue,
    "Cooldown": Cooldown,
    "StatusEffects": StatusEffects,
    "CombatTarget": CombatTarget,
    "LootTable": LootTable,
    "CameraShake": CameraShake,
    "Light2D": Light2D,
    "ParallaxLayer": ParallaxLayer,
    "TilemapCollider": TilemapCollider,
    "ObjectiveMarker": ObjectiveMarker,
    "Checkpoint": Checkpoint,
    "CharacterController2D": CharacterController2D,
    "EconomyWallet": EconomyWallet,
    "Timer": Timer,
    "Tween": Tween,
}


ADVANCED_COMPONENT_CATEGORIES = {
    "Stats": "Gameplay",
    "Inventory": "Gameplay",
    "Equipment": "Gameplay",
    "Ability": "Gameplay",
    "AIController": "AI",
    "NavAgent": "Navigation",
    "Interaction": "Gameplay",
    "Lifetime": "Gameplay",
    "Spawner": "Gameplay",
    "DamageDealer": "Combat",
    "CameraFollow": "Camera",
    "Saveable": "Persistence",
    "Blackboard": "Scripting",
    "StateMachine": "Scripting",
    "QuestLog": "Narrative",
    "Dialogue": "Narrative",
    "Cooldown": "Gameplay",
    "StatusEffects": "Combat",
    "CombatTarget": "Combat",
    "LootTable": "Gameplay",
    "CameraShake": "Camera",
    "Light2D": "Rendering",
    "ParallaxLayer": "Rendering",
    "TilemapCollider": "Physics",
    "ObjectiveMarker": "UI",
    "Checkpoint": "Gameplay",
    "CharacterController2D": "Gameplay",
    "EconomyWallet": "Gameplay",
    "Timer": "Scripting",
    "Tween": "Scripting",
}


def component_from_data(data):
    component_type = data.get("component_type")

    if component_type == "Transform":
        component = Transform(
            data.get("x", 0),
            data.get("y", 0),
            data.get("rotation", 0),
            data.get("scale_x", 1),
            data.get("scale_y", 1),
        )

    elif component_type == "SpriteRenderer":
        component = SpriteRenderer(
            data.get("sprite_name"),
            data.get("visible", True),
            data.get("sorting_order", 0),
        )
        component.sprite_guid = data.get("sprite_guid")
        component.flip_x = data.get("flip_x", False)
        component.flip_y = data.get("flip_y", False)
        component.tint = tuple(data.get("tint", (255, 255, 255)))

    elif component_type == "RTSMovement":
        component = RTSMovement(data.get("speed", 3.5))
        component.separation = data.get("separation", True)
        component.allow_pathfinding = data.get("allow_pathfinding", True)
        component.formation_role = data.get("formation_role", "unit")
        component.acceleration = data.get("acceleration", 1.0)
        component.turn_speed = data.get("turn_speed", 1.0)

    elif component_type == "Selectable":
        component = Selectable(data.get("selectable", True))
        component.selection_radius = data.get("selection_radius", 0.5)

    elif component_type == "AudioSource":
        component = AudioSource(
            data.get("audio_name"),
            data.get("volume", 1.0),
            data.get("play_on_start", False),
        )
        component.pitch = data.get("pitch", 1.0)
        component.bus = data.get("bus", "SFX")
        component.spatial_blend = data.get("spatial_blend", 0.0)
        component.min_distance = data.get("min_distance", 4.0)
        component.max_distance = data.get("max_distance", 18.0)
        component.loop = data.get("loop", False)
        component.priority = data.get("priority", 128)

    elif component_type == "Rigidbody2D":
        component = Rigidbody2D(data.get("body_type", "dynamic"))
        component.velocity_x = data.get("velocity_x", 0.0)
        component.velocity_y = data.get("velocity_y", 0.0)
        component.angular_velocity = data.get("angular_velocity", 0.0)
        component.mass = data.get("mass", 1.0)
        component.gravity_scale = data.get("gravity_scale", 1.0)
        component.drag = data.get("drag", 0.05)
        component.angular_drag = data.get("angular_drag", 0.05)
        component.bounciness = data.get("bounciness", 0.0)
        component.friction = data.get("friction", 0.25)
        component.use_gravity = data.get("use_gravity", True)
        component.freeze_x = data.get("freeze_x", False)
        component.freeze_y = data.get("freeze_y", False)
        component.freeze_rotation = data.get("freeze_rotation", False)
        component.continuous_collision = data.get("continuous_collision", False)
        component.sleeping = data.get("sleeping", False)

    elif component_type == "Animator":
        component = Animator()
        component.controller = data.get("controller", "Default")
        component.current_state = data.get("current_state", "Idle")
        component.speed = data.get("speed", 1.0)
        component.play_on_start = data.get("play_on_start", True)
        component.loop = data.get("loop", True)
        component.parameters = data.get("parameters", {})
        component.preview = data.get("preview", True)
        component.apply_sprite = data.get("apply_sprite", True)
        component.apply_tint = data.get("apply_tint", True)
        component.normalized_time = data.get("normalized_time", 0.0)

    elif component_type == "VisualScript":
        component = VisualScript()
        component.graph_name = data.get("graph_name", "NewGraph")
        component.run_in_editor = data.get("run_in_editor", False)
        component.variables = data.get("variables", {})
        component.nodes = data.get("nodes", component.nodes)
        component.enabled_events = data.get("enabled_events", component.enabled_events)

    elif component_type == "UIElement":
        component = UIElement(data.get("element_type", "Label"))
        component.text = data.get("text", "Label")
        component.anchor = data.get("anchor", "top_left")
        component.x = data.get("x", 24.0)
        component.y = data.get("y", 24.0)
        component.width = data.get("width", 160.0)
        component.height = data.get("height", 36.0)
        component.color = tuple(data.get("color", (245, 247, 252)))
        component.text_color = tuple(data.get("text_color", (35, 36, 42)))
        component.image_name = data.get("image_name")
        component.opacity = data.get("opacity", 1.0)
        component.interactable = data.get("interactable", component.element_type == "Button")
        component.on_click_graph = data.get("on_click_graph")
        component.sorting_order = data.get("sorting_order", 0)
        component.padding = data.get("padding", 8)
        component.border_radius = data.get("border_radius", 7)
        component.border_color = tuple(data.get("border_color", (180, 185, 198)))
        component.text_align = data.get("text_align", "center")
        component.font_size = data.get("font_size", 0)
        component.progress = data.get("progress", 1.0)
        component.max_progress = data.get("max_progress", 1.0)

    elif component_type == "Collider2D":
        component = Collider2D(
            data.get("shape", "rect"),
            data.get("width", 1.0),
            data.get("height", 1.0),
            data.get("radius", 0.5),
            data.get("is_trigger", False),
        )
        component.offset_x = data.get("offset_x", 0.0)
        component.offset_y = data.get("offset_y", 0.0)

    elif component_type == "Health":
        component = Health(
            data.get("max_health", 100),
            data.get("armor", 0),
        )
        component.health = data.get("health", component.max_health)
        component.alive = data.get("alive", True)

    elif component_type == "Team":
        component = Team(
            data.get("team_id", 0),
            data.get("team_name", "Neutral"),
            tuple(data.get("color", (80, 120, 255))),
        )

    elif component_type == "ResourceNode":
        component = ResourceNode(
            data.get("resource_type", "Gold"),
            data.get("amount", 500),
            data.get("gather_rate", 10),
        )
        component.max_amount = data.get("max_amount", component.amount)

    elif component_type == "Worker":
        component = Worker(data.get("carry_capacity", 50))
        component.carrying_type = data.get("carrying_type")
        component.carrying_amount = data.get("carrying_amount", 0)
        component.gather_target_id = data.get("gather_target_id")

    elif component_type in ADVANCED_COMPONENT_TYPES:
        component = ADVANCED_COMPONENT_TYPES[component_type]()
        component.deserialize_fields(data)

    else:
        return None

    component.deserialize_base(data)
    return component
