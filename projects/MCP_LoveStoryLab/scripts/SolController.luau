--!strict

function on_start()
    set_sprite("assets/sprites/LoveLabCharacters.sprite.json")
    play_sprite_animation("assets/animations/LoveLabCharacters.spriteframes", "idle")
end

function on_update(_dt: number)
    if input_pressed("A") then
        face_left()
    elseif input_pressed("D") then
        face_right()
    end
end

function on_collision_enter(other: string)
    set_ui_text("HUD_Status", "Sol touched " .. other)
end
