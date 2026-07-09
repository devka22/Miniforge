--!strict

function on_start()
    set_sprite("assets/sprites/LoveLabCharacters.sprite.json")
    play_sprite_animation("assets/animations/LoveLabCharacters.spriteframes", "idle")
end

function on_update(dt: number)
    if input_pressed("A") then
        face_left()
        move(-3.5 * dt, 0)
    elseif input_pressed("D") then
        face_right()
        move(3.5 * dt, 0)
    end
end
