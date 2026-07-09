--!strict

function on_start()
    set_sprite("assets/sprites/LoveLabCharacters.sprite.json")
    play_sprite_animation("assets/animations/LoveLabCharacters.spriteframes", "idle")
end

function on_event(name: string, payload: any)
    if name == "dialogue_focus" and type(payload) == "table" then
        set_ui_text("HUD_Status", "Talking with " .. tostring(payload.speaker))
    end
end
