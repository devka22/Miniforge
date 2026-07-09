--!strict
-- MiniForge Luau script: EnemyBrain
-- Attach this file with ScriptComponent.path = "EnemyBrain.luau".

local EnemyBrain = {}

function EnemyBrain:on_create()
    self.speed = self.speed or 220
    -- play_sound("spawn")
    -- set_sprite("assets/sprites/player.sprite.json")
end

function EnemyBrain:on_ready()
end

function EnemyBrain:on_update(dt: number)
    local direction = Input.get_axis("A", "D")
    if direction < 0 then
        face_left()
    elseif direction > 0 then
        face_right()
    end
    self.entity.transform.position.x += direction * self.speed * dt
end

function EnemyBrain:on_fixed_update(dt: number)
end

function EnemyBrain:on_collision_enter(other: string)
    -- ui_text("Hit " .. other)
end

function EnemyBrain:on_collision_exit(other: string)
end

function EnemyBrain:on_destroy()
end

return EnemyBrain
