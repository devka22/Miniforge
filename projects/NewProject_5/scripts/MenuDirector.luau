local CityDB = require("./CityDB")

local Menu = {
    visible = false,
    cooldown = 0,
    refresh_timer = 0,
    pulse = 0,
}

local elements = {
    "Menu_Backdrop",
    "Menu_Title",
    "Menu_Subtitle",
    "Menu_Command",
    "Menu_Deco_1",
    "Menu_Deco_2",
}

function Menu:set_visible(value)
    self.visible = value
    for _, name in ipairs(elements) do
        set_ui_visible_for(name, value)
    end
end

function Menu:on_start()
    self:set_visible(false)
    self:refresh()
end

local function city_values()
    return Component.get("CityDirector", "Blackboard", "values", {})
end

function Menu:refresh()
    local city = city_values()
    local mission_title = city.mission_title or "NEON SOMBRA"
    local objective = city.mission_waypoint and ("Destino: " .. city.mission_waypoint) or "Continuar historia"
    local district = city.district or "Santa Aurelia"
    local heat = city.heat_label or CityDB.dispatch_label(city.wanted or 0)
    set_ui_text("Menu_Title", "NEON SOMBRA")
    set_ui_text("Menu_Subtitle", mission_title)
    set_ui_text("Menu_Command", district .. " / " .. heat)
    set_ui_text("Menu_Deco_1", objective .. "  |  Rep " .. tostring(city.reputation or 0))
    set_ui_text("Menu_Deco_2", CityDB.menu_motto(city.mission_index or 1, city.wanted or 0))
end

function Menu:on_update(dt)
    self.cooldown = math.max(0, self.cooldown - dt)
    self.refresh_timer = math.max(0, self.refresh_timer - dt)
    self.pulse = self.pulse + dt
    if self.cooldown <= 0 and Input.action_pressed("pause") then
        self.cooldown = 0.4
        self:set_visible(not self.visible)
    end
    if self.visible and self.refresh_timer <= 0 then
        self.refresh_timer = 0.25
        self:refresh()
        local glow = 0.42 + math.sin(self.pulse * 2.8) * 0.08
        Component.set("Menu_Title", "UIElement", "opacity", glow)
        Component.set("Menu_Command", "UIElement", "opacity", 0.68 + glow * 0.28)
    end
end

function Menu:on_event(name, payload)
    if name == "mission_started" and self.visible then
        self:refresh()
    elseif (name == "wanted_changed" or name == "mission_completed" or name == "witness_reported") and self.visible then
        self:refresh()
    end
end

return Menu
