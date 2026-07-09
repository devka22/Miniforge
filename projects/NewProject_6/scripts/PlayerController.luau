--!strict
-- MiniForge Luau script: PlayerController
-- Attach this file with ScriptComponent.path = "PlayerController.luau".

local PlayerController = {}

function PlayerController:on_create()
    self.speed = self.speed or 220
    -- play_sound("spawn")
    -- set_sprite("assets/sprites/player.sprite.json")
end

function PlayerController:on_ready()
end

function PlayerController:on_update(dt: number)
    local direction = Input.get_axis("A", "D")
    if direction < 0 then
        face_left()
    elseif direction > 0 then
        face_right()
    end
    self.entity.transform.position.x += direction * self.speed * dt
end

function PlayerController:on_fixed_update(dt: number)
end

function PlayerController:on_collision_enter(other: string)
    -- ui_text("Hit " .. other)
end

function PlayerController:on_collision_exit(other: string)
end

function PlayerController:on_destroy()
end

return PlayerController
