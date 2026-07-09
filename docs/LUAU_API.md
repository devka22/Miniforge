# MiniForge Luau API

MiniForge keeps Rust as the engine/runtime core and runs Luau as gameplay scripting. Attach scripts with `ScriptComponent.path`, `ScriptComponent.scripts`, `entity.script`, or `entity.scripts`. New code should prefer `ScriptComponent`.

## Script Shape

```luau
local Player = {}

function Player:on_create()
    self.speed = self.speed or 220
end

function Player:on_update(dt)
    local direction = Input.get_axis("move_left", "move_right")
    self.entity.transform.position.x += direction * self.speed * dt
end

return Player
```

Legacy global callbacks such as `function on_start()` and helpers such as `move()` still work for compatibility.

## Lifecycle

Callbacks are protected: runtime errors are reported with script path, callback name, line/stack information from Luau, and the engine continues running.

- `on_create()`
- `on_ready()`
- `on_update(dt)`
- `on_fixed_update(dt)`
- `on_collision_enter(other)`
- `on_collision_exit(other)`
- `on_destroy()`

## Safe Runtime API

- `self.entity`: copy of the current entity state. Mutating `self.entity.transform.position`, `rotation`, `scale`, `size`, `tag`, `layer`, `enabled`, or `visible` is copied back after the callback.
- `Vector2.new(x, y)`, `Vector2.length(v)`, `Vector2.normalized(v)`.
- `Input.is_pressed(key)`, `Input.get_axis(negative, positive)`.
- `Time.delta_time`, `Time.fixed_delta_time`, `Time.time`, `Time.frame`.
- `Entity.spawn(name, x, y)`, `Entity.destroy(target)`.
- `Transform2D.set_position(entity, x, y)`, `Transform2D.translate(entity, dx, dy)`.
- `Scene.load(name)`.
- `Events.emit(name, payload)`.
- `Assets.exists(path)`, restricted to the project tree.
- `Debug.log(value)`, `Debug.warn(value)`, `Debug.error(value)`.

`require("./Module")` loads `.luau`/`.lua` modules from the project `scripts/` folder only and caches module results.

## Exported Variables

Primitive fields assigned on `self` are copied into `ScriptComponent.public_variables` after callbacks and shown in the Inspector. Inspector edits are applied back to `self` before the next callback.

Type definitions for autocomplete live in `types/miniforge.luau`.
