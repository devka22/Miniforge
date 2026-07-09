--!strict
-- Narrative orchestration for Letters Under Rain.

local affection = 35
local honesty = 40
local courage = 20

local function refresh_score()
    set_ui_progress_for("HUD_Affection", affection, 100)
    set_ui_text("HUD_Affection", "Affection " .. tostring(affection))
    set_blackboard("affection", affection)
    set_blackboard("honesty", honesty)
    set_blackboard("courage", courage)
end

function on_start()
    set_ui_text("HUD_Status", "ACT I · Meet Mara, recover the letter, then choose at the bridge.")
    set_ui_text("HUD_Dialogue", "Rain taps on the platform roof. Mara is waiting.")
    refresh_score()
    play_sound("RainTheme", "Music", 0.72, true)
end

function on_key_down(key: string)
    if key == "Space" then
        set_ui_text("HUD_Dialogue", "MEMORY · The letter was written on the night the trains stopped.")
        set_ui_text("HUD_Status", "A memory surfaced · find the unsent letter.")
        play_sound("HeartbeatChoice", "SFX", 0.65, false)
    elseif key == "1" then
        set_blackboard("last_input_choice", "honest")
    elseif key == "2" then
        set_blackboard("last_input_choice", "gentle")
    end
end

function on_event(name: string, payload: any)
    if name == "story_score" and type(payload) == "table" then
        affection = payload.affection or affection
        honesty = payload.honesty or honesty
        courage = payload.courage or courage
        refresh_score()
    end
end
