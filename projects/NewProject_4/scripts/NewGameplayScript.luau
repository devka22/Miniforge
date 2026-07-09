--!strict
-- MiniForge Luau script: NewGameplayScript
-- Attach this file to an entity with script = "NewGameplayScript.luau".

function on_start()
    -- play_sound("spawn")
    -- set_sprite("assets/sprites/player.sprite.json")
end

function on_update(dt: number)
    if input_pressed("A") then
        face_left()
        move(-4.0 * dt, 0.0)
    end
    if input_pressed("D") then
        face_right()
        move(4.0 * dt, 0.0)
    end
end

function on_key_down(key: string)
    if key == "Space" then
        play_sound("jump")
    end
end

function on_collision_enter(other: string)
    -- ui_text("Hit " .. other)
end

function on_destroy()
end
