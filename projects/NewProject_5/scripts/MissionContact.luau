local CityDB = require("./CityDB")

local Contact = {
    pulse = 0,
    last_line = "",
}

local function values(entity)
    return Component.get(entity, "Blackboard", "values", {})
end

function Contact:ensure_data()
    local data = values(self.entity)
    self.contact_id = self.contact_id or data.contact_id or self.entity.name
    self.display_name = self.display_name or data.display_name or self.contact_id
    self.line = self.line or data.line or "..."
end

function Contact:on_start()
    self:ensure_data()
    Component.set(self.entity, "Light2D", "flicker", true)
    AnimationPlayer.play(self.entity, "Idle")
end

function Contact:on_update(dt)
    self:ensure_data()
    self.pulse = self.pulse + dt
    local nearby = Entity.nearby(self.entity, 2.2, { tag = "Player" })
    if #nearby > 0 then
        local city = values("CityDirector")
        local line = CityDB.contact_line(self.contact_id, city.mission_index or 1, city.reputation or 0)
        self.last_line = line
        set_ui_text("HUD_Prompt", self.display_name .. ": " .. line)
        Component.set(self.entity, "Light2D", "intensity", 1.55 + math.sin(self.pulse * 4.0) * 0.15)
        Component.set(self.entity, "Blackboard", "values", {
            contact_id = self.contact_id,
            display_name = self.display_name,
            line = self.line,
            last_line = line,
            story_contact = true,
        })
    else
        Component.set(self.entity, "Light2D", "intensity", 1.0)
    end
end

function Contact:on_event(name, payload)
    self:ensure_data()
    if name == "mission_started" and payload.waypoint == self.entity.name then
        set_ui_text("HUD_Dialogue", "Objetivo marcado: " .. self.display_name)
    elseif name == "contact_interact" and payload.contact_name == self.entity.name then
        local character = CityDB.characters[self.contact_id]
        if character then
            local city = values("CityDirector")
            local line = CityDB.contact_line(self.contact_id, city.mission_index or 1, city.reputation or 0)
            if (city.reputation or 0) >= 3 and character.trust_line then
                line = character.trust_line
            end
            self.last_line = line
            set_ui_text("HUD_Dialogue", self.display_name .. ": " .. line)
        end
    end
end

return Contact
