local SavePoint = {
    cooldown = 0,
    pulse = 0,
    saved_timer = 0,
}

local function values(entity)
    return Component.get(entity, "Blackboard", "values", {})
end

function SavePoint:on_start()
    local data = values(self.entity)
    self.label = data.label or self.entity.name
    self.slot = data.slot or "autosave"
    Component.set(self.entity, "Light2D", "flicker", true)
    AnimationPlayer.play(self.entity, "Idle")
end

function SavePoint:on_update(dt)
    self.cooldown = math.max(0, self.cooldown - dt)
    self.saved_timer = math.max(0, self.saved_timer - dt)
    self.pulse = self.pulse + dt

    local nearby = Entity.nearby(self.entity, 1.7, { tag = "Player" })
    if #nearby == 0 then
        Component.set(self.entity, "Light2D", "intensity", 0.85 + math.sin(self.pulse * 2.0) * 0.08)
        return
    end

    Component.set(self.entity, "Light2D", "intensity", 1.45 + math.sin(self.pulse * 5.0) * 0.18)
    if self.saved_timer > 0 then
        set_ui_text("HUD_Prompt", "Guardado: " .. self.label)
        Component.set("HUD_Prompt", "UIElement", "opacity", 0.88)
        return
    end

    set_ui_text("HUD_Prompt", "E: guardar en " .. self.label)
    Component.set("HUD_Prompt", "UIElement", "opacity", 0.88)
    if Input.action_pressed("interact") and self.cooldown <= 0 then
        self.cooldown = 1.0
        self.saved_timer = 2.5
        Component.set(self.entity, "Checkpoint", "active", true)
        Component.set(self.entity, "Blackboard", "values", {
            label = self.label,
            slot = self.slot,
            saved = true,
        })
        Game.save_slot(self.slot)
        Events.emit("game_saved", {
            slot = self.slot,
            save_point = self.entity.name,
            label = self.label,
        })
        set_ui_text("HUD_Radio", "Sistema: partida guardada en " .. self.label)
        Audio2D.play("save_point", { volume = 0.52, bus = "UI" })
    end
end

return SavePoint
