local CityDB = require("./CityDB")

local Director = {
    wanted = 0,
    time_of_day = 20.35,
    mission_index = 1,
    radio_timer = 2.0,
    save_timer = 0,
    weather_timer = 12.0,
    weather_index = 1,
    weather = nil,
    day_phase = "night",
    night_factor = 0.78,
    headlights = true,
    blockade_level = 0,
    reputation = 0,
    current_district = "Bajo Muelle",
    district_mood = "sal, oxido y favores viejos",
    ui_pulse = 0,
}

local function clamp(value, low, high)
    if value < low then
        return low
    end
    if value > high then
        return high
    end
    return value
end

local function two_digit(value)
    local n = math.floor(value)
    if n < 10 then
        return "0" .. tostring(n)
    end
    return tostring(n)
end

local function clock_text(hour_float)
    local hour = math.floor(hour_float) % 24
    local minute = math.floor((hour_float - math.floor(hour_float)) * 60)
    return two_digit(hour) .. ":" .. two_digit(minute)
end

local function phase_for(hour_float)
    local hour = math.floor(hour_float) % 24
    if hour >= 5 and hour < 8 then
        return "dawn"
    elseif hour >= 8 and hour < 18 then
        return "day"
    elseif hour >= 18 and hour < 21 then
        return "dusk"
    end
    return "night"
end

local function night_factor_for(hour_float)
    local radians = (hour_float % 24) / 24.0 * math.pi * 2.0
    return clamp(0.5 + 0.5 * math.cos(radians), 0.0, 1.0)
end

function Director:weather_name()
    return (self.weather and self.weather.name) or "Claro"
end

function Director:weather_id()
    return (self.weather and self.weather.id) or "clear"
end

function Director:weather_roll()
    local count = #CityDB.weather
    if count <= 1 then
        self.weather_index = 1
    else
        local seed = math.floor(self.time_of_day * 10) + self.mission_index * 3 + self.weather_index * 5 + math.floor(self.wanted)
        self.weather_index = (seed % count) + 1
    end
    self.weather = CityDB.weather_at(self.weather_index)
    self.weather_timer = 28.0 + ((self.weather_index * 11 + self.mission_index * 7) % 22)
    Events.emit("weather_changed", {
        id = self:weather_id(),
        name = self:weather_name(),
        rain = self.weather.rain or 0,
        fog = self.weather.fog or 0,
        traffic = self.weather.traffic or 1,
        headlights = self.weather.headlights or false,
    })
    set_ui_text("HUD_Radio", "Clima: " .. self.weather.name .. ", " .. self.weather.radio)
end

function Director:update_world_simulation(dt)
    self.time_of_day = self.time_of_day + dt * 0.018
    if self.time_of_day >= 24 then
        self.time_of_day = self.time_of_day - 24
    end

    self.weather_timer = self.weather_timer - dt
    if not self.weather or self.weather_timer <= 0 then
        self:weather_roll()
    end

    local phase = phase_for(self.time_of_day)
    if phase ~= self.day_phase then
        self.day_phase = phase
        Events.emit("time_phase_changed", {
            phase = phase,
            time_of_day = self.time_of_day,
        })
        if phase == "night" then
            set_ui_text("HUD_Radio", "La noche prende la ciudad. Los coches encienden luces.")
        elseif phase == "day" then
            set_ui_text("HUD_Radio", "Amanecio en Santa Aurelia. Las sombras se hacen mas cortas.")
        end
    end

    self.night_factor = night_factor_for(self.time_of_day)
    self.headlights = self.night_factor > 0.38 or (self.weather and self.weather.headlights) or false
end

function Director:publish_state()
    local mission = CityDB.mission_at(self.mission_index)
    local weather = self.weather or CityDB.weather_at(self.weather_index)
    Component.set(self.entity, "Blackboard", "values", {
        wanted = self.wanted,
        reputation = self.reputation,
        mission_index = self.mission_index,
        mission_id = mission.id,
        mission_title = mission.title,
        mission_waypoint = mission.waypoint,
        time_of_day = self.time_of_day,
        phase = self.day_phase,
        night_factor = self.night_factor,
        weather = weather.id,
        weather_name = weather.name,
        rain = weather.rain or 0,
        fog = weather.fog or 0,
        traffic_factor = weather.traffic or 1,
        headlights = self.headlights,
        district = self.current_district,
        district_mood = self.district_mood,
        heat_label = CityDB.dispatch_label(self.wanted),
    })
    set_ui_text("HUD_Mission", mission.title)
    set_ui_text("HUD_Objective", mission.objective)
    set_ui_text("HUD_Clock", clock_text(self.time_of_day))
    set_ui_text("HUD_Weather", self.day_phase .. " / " .. weather.name)
    set_ui_text("HUD_Wanted", "Busqueda " .. tostring(math.floor(self.wanted + 0.5)))
    set_ui_text("HUD_Heat", CityDB.dispatch_label(self.wanted))
    set_ui_text("HUD_District", self.current_district)
    set_ui_text("HUD_Reputation", "Rep " .. tostring(self.reputation))
    set_ui_text("Menu_Command", mission.title)
    set_ui_text("Menu_Subtitle", self.current_district .. " / " .. CityDB.dispatch_label(self.wanted))
    set_ui_text("Menu_Deco_1", mission.objective)
    set_ui_text("Menu_Deco_2", CityDB.menu_motto(self.mission_index, self.wanted))
end

function Director:update_player_context()
    local player = Entity.find("Player")
    if not player then
        return
    end
    local position = player.transform.position
    local district = CityDB.district_at(position.x, position.y)
    if district.name ~= self.current_district then
        self.current_district = district.name
        self.district_mood = district.mood
        set_ui_text("HUD_Dialogue", district.name .. ": " .. district.rumor)
        self:publish_state()
    else
        self.district_mood = district.mood
    end
end

function Director:style_ui(dt)
    self.ui_pulse = self.ui_pulse + dt
    local wanted = math.floor(self.wanted + 0.5)
    local pulse = 0.5 + math.sin(self.ui_pulse * (2.0 + wanted)) * 0.5
    Component.set("HUD_Wanted", "UIElement", "opacity", 0.78 + pulse * 0.18)
    Component.set("HUD_Heat", "UIElement", "opacity", 0.68 + pulse * 0.18)
    if self.wanted >= 3.0 then
        Component.set("HUD_Wanted", "UIElement", "text_color", { 255, 120, 135 })
        Component.set("Menu_Title", "UIElement", "text_color", { 140, 190, 255 })
        Component.set("Menu_Backdrop", "UIElement", "color", { 18, 14, 24 })
    else
        Component.set("HUD_Wanted", "UIElement", "text_color", { 255, 205, 218 })
        Component.set("Menu_Title", "UIElement", "text_color", { 120, 222, 255 })
        Component.set("Menu_Backdrop", "UIElement", "color", { 10, 16, 26 })
    end
end

function Director:set_roadblocks(level)
    if self.blockade_level == level then
        return
    end
    self.blockade_level = level
    local active_count = 0
    if level >= 3 then
        active_count = 3
    end
    if level >= 4 then
        active_count = 5
    end
    if level >= 5 then
        active_count = 9
    end

    for index = 1, 9 do
        local name = "Roadblock_" .. (index < 10 and ("0" .. tostring(index)) or tostring(index))
        local active = index <= active_count
        Entity.set_enabled(name, active)
        Entity.set_visible(name, active)
    end
    if active_count > 0 then
        set_ui_text("HUD_Radio", "Central: bloqueos activos en " .. tostring(active_count) .. " cruces.")
    else
        set_ui_text("HUD_Radio", "Central: se levantan los bloqueos. Patrullas vuelven a ruta.")
    end
end

function Director:on_start()
    self.weather = CityDB.weather_at(self.weather_index)
    self.day_phase = phase_for(self.time_of_day)
    self.night_factor = night_factor_for(self.time_of_day)
    self.headlights = self.night_factor > 0.38 or (self.weather and self.weather.headlights) or false
    self:update_player_context()
    self:publish_state()
    self:set_roadblocks(0)
    set_ui_text("HUD_Dialogue", CityDB.mission_at(self.mission_index).beat)
    set_ui_text("Menu_Title", "NEON SOMBRA")
    set_ui_text("Menu_Subtitle", "Santa Aurelia / modo historia")
    set_ui_text("Menu_Command", "Continuar historia")
    Events.emit("mission_started", CityDB.mission_at(self.mission_index))
end

function Director:on_update(dt)
    self:update_world_simulation(dt)

    local weather = self.weather or CityDB.weather_at(self.weather_index)
    local weather_ambient = weather.ambient or 1.0
    local weather_bloom = weather.bloom or 0.0
    local ambient = (0.50 + self.night_factor * 0.52) * weather_ambient
    Component.set("World_Ambient", "Light2D", "intensity", ambient)
    Component.set("World_Ambient", "Light2D", "color", self.night_factor > 0.45 and { 91, 117, 174 } or { 180, 198, 206 })
    Component.set("World_Bloom", "Bloom2D", "intensity", 0.40 + self.wanted * 0.08 + weather_bloom + self.night_factor * 0.12)
    self:style_ui(dt)

    if self.wanted > 0 then
        self.wanted = math.max(0, self.wanted - CityDB.dispatch.decay_per_second * dt)
    end
    self:set_roadblocks(math.floor(self.wanted + 0.001))
    self:update_player_context()

    self.radio_timer = self.radio_timer - dt
    if self.radio_timer <= 0 then
        self.radio_timer = 9.0 + (self.mission_index * 1.7)
        set_ui_text("HUD_Radio", CityDB.radio_line(self.wanted, self.mission_index, self.current_district))
    end

    self.save_timer = self.save_timer + dt
    if self.save_timer > 0.5 then
        self.save_timer = 0
        self:publish_state()
    end
end

function Director:on_event(name, payload)
    if name == "crime_committed" then
        self.wanted = clamp(self.wanted + 0.75 + ((payload.wanted or 0) * 0.15), 0, 5)
        self:publish_state()
        self:set_roadblocks(math.floor(self.wanted + 0.001))
        Events.emit("wanted_changed", {
            wanted = self.wanted,
            x = payload.x,
            y = payload.y,
            kind = payload.kind,
        })
        set_ui_text("HUD_Dialogue", "Central: incidente reportado. " .. CityDB.dispatch_label(self.wanted))
    elseif name == "witness_reported" then
        self.wanted = clamp(self.wanted + 0.12 + ((payload.confidence or 0) * 0.08), 0, 5)
        self:publish_state()
        Events.emit("wanted_changed", {
            wanted = self.wanted,
            x = payload.x,
            y = payload.y,
            kind = "witness_report",
        })
        local district = payload.district or self.current_district
        local index = ((self.mission_index + math.floor(self.wanted * 4)) % #CityDB.witness_lines) + 1
        set_ui_text("HUD_Radio", string.format(CityDB.witness_lines[index], district))
    elseif name == "contact_interact" then
        local mission = CityDB.mission_at(self.mission_index)
        local expected = mission.waypoint
        if payload.contact_name == expected then
            set_ui_text("HUD_Dialogue", mission.beat)
            Events.emit("mission_completed", {
                id = mission.id,
                title = mission.title,
                reward = 180 + self.mission_index * 40,
            })
            self.reputation = self.reputation + 1
            self.mission_index = math.min(#CityDB.missions, self.mission_index + 1)
            self:publish_state()
            Events.emit("mission_started", CityDB.mission_at(self.mission_index))
        else
            set_ui_text("HUD_Dialogue", "No es el contacto correcto. La ciudad habla, pero no siempre ayuda.")
        end
    elseif name == "pickup_collected" then
        set_ui_text("HUD_Dialogue", "Recuperado: " .. tostring(payload.pickup_name))
    elseif name == "game_saved" then
        set_ui_text("HUD_Dialogue", "Guardado en " .. tostring(payload.label or payload.save_point or "punto seguro") .. ". Santa Aurelia recuerda.")
    elseif name == "player_arrested" then
        self.wanted = 0
        self:publish_state()
        self:set_roadblocks(0)
        set_ui_text("HUD_Dialogue", "Rojas gana esta ronda. Te sueltan lejos del centro, sin ruido y sin orgullo.")
        Events.emit("wanted_changed", { wanted = 0 })
    end
end

return Director
