# MiniForge Luau 2D API

Rust owns simulation, rendering-facing state and serialization. Luau owns gameplay decisions through a safe command API. Scripts enqueue commands during callbacks; Rust applies them after the callback returns.

Attach scripts with `entity.script = "PlayerController.luau"` or a `ScriptComponent`.

## Lifecycle

```luau
local Player = {}

function Player:on_start() end
function Player:on_update(dt) end
function Player:on_fixed_update(dt) end
function Player:on_collision_enter(other_name) end
function Player:on_collision_exit(other_name) end
function Player:on_event(name, payload) end
function Player:on_destroy() end

return Player
```

Global function style (`function on_update(dt)`) is still supported.

## Physics2D

Components: `Rigidbody2D`, `CharacterBody2D`, `StaticBody2D`, `Area2D`, `Collider2D`, `OneWayPlatform2D`, `TilemapCollider`.

```luau
Rigidbody2D.set_velocity(self.entity, 6, 0)
Rigidbody2D.apply_impulse(self.entity, 0, -8)
CharacterBody2D.move(self.entity, move_x, 0, jump, run)

local hit = Physics2D.raycast(origin, target, {
    mask = Layers.ENEMY,
    include_triggers = true,
})

local cast = Physics2D.shape_cast(origin, target, {
    shape = "circle",
    radius = 0.35,
    mask = { Layers.ENEMY, Layers.WORLD_STATIC },
})

local overlaps = Physics2D.overlap_area(origin, Vector2.new(1, 1), {
    mask = Layers.TRIGGER,
})
```

Collision callbacks receive legacy names through `on_collision_enter/exit`. Structured events also arrive through `on_event` as `physics_collision_enter`, `physics_collision_exit`, `physics_trigger_enter`, or `physics_trigger_exit`.

## Animation 2D

Components: `AnimatedSprite`, `AnimationPlayer`, `Animator`, `Animator2D`, `AnimationBlueprint2D`, `StateMachine`.

```luau
AnimationPlayer.play(self.entity, "Run")
AnimationPlayer.set_parameter(self.entity, "grounded", true)
Component.set(self.entity, "AnimatedSprite", "speed", 1.25)
```

Rust advances clips/state transitions and writes sampled sprite/property state.

## Tilemap

Components/assets: `Tilemap2D`, `TilemapRenderer2D`, `TilemapCollider`, `Tileset2D`.

```luau
local tile = Tilemap.get_tile("LevelTilemap", "Collision", 4, 12)
Tilemap.set_tile("LevelTilemap", "Collision", 4, 12, 1)
```

`Tilemap2D` supports multiple layers, chunk metadata, collision/navigation flags, autotile metadata and animated tile metadata. Edits mark dirty chunks for editor/runtime refresh.

## Camera 2D

Components: `Camera2D`, `CameraFollow`, `CameraShake`.

```luau
Camera.main():follow(self.entity)
Camera.main():set_zoom(2.0)
Camera.main():set_limits(0, 0, 384, 224)
Camera.main():pixel_perfect(true, 16)
Camera.main():shake(0.3, 6)

local screen = Camera.main():world_to_screen(self.entity.transform.position)
local world = Camera.main():screen_to_world(screen)
```

## Gameplay

Input:

```luau
local x = Input.axis("move_left", "move_right")
if Input.action_pressed("fire") then end
```

Events and timers:

```luau
Events.emit("enemy_defeated", { score = 100 })

Task.delay(1.5, function()
    destroy()
end)
```

Spawning:

```luau
Entity.spawn("Projectile", x, y, {
    tag = "Projectile",
    script = "Projectile.luau",
    components = {
        { component_type = "Rigidbody2D", body_type = "kinematic", velocity_x = 14 },
        { component_type = "Collider2D", collision_layer = "Projectile", collision_mask = {"Enemy"} },
        { component_type = "Lifetime", duration = 1.2 },
    },
})
```

Tweens/navigation/audio/particles:

```luau
Tween.to(self.entity, "scale_x", 1.2, 0.1, { easing = "ease_out" })
Navigation2D.set_destination(self.entity, 10, 4)
Audio2D.play("jump", { volume = 0.8, bus = "SFX" })
Particles2D.burst(self.entity, 12)
```

Generic component bridge:

```luau
Component.add(self.entity, "Health", { max_health = 100, health = 100 })
Component.set(self.entity, "Health", "health", 75)
```

See `examples/luau_2d_showcase` for a complete scene using movement, physics, collisions, animation, camera, projectiles, enemies, events, timers, tilemaps and hot reload.
