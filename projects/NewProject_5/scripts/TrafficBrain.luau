local Traffic = {
    route_index = 1,
    rethink = 0,
    horn_timer = 0,
    occupied = false,
    driver = nil,
    drive_speed = 8.5,
    resume_timer = 0,
    state = "route",
    last_seen_x = nil,
    last_seen_y = nil,
}

local function values(entity)
    return Component.get(entity, "Blackboard", "values", {})
end

local function city_state()
    return values("CityDirector")
end

local function lights_needed(city)
    return city.headlights == true or (city.night_factor or 0) > 0.38 or (city.rain or 0) > 0.25 or (city.fog or 0) > 0.35
end

function Traffic:on_start()
    local data = values(self.entity)
    self.route = data.route or {}
    self.route_index = data.route_index or 1
    self.occupied = data.occupied or false
    self.drive_speed = data.drive_speed or 8.5
    self.display_name = data.display_name or self.entity.name
    Component.set(self.entity, "NavAgent", "speed", data.speed or 4.2)
    self:apply_lighting(city_state(), self.entity.tag == "Police" and 1.4 or 0.75, false)
    AnimationPlayer.play(self.entity, "Drive")
end

function Traffic:apply_lighting(city, base, alert)
    city = city or {}
    local wanted = city.wanted or 0
    local rain = city.rain or 0
    local fog = city.fog or 0
    local night = city.night_factor or 0
    local needed = lights_needed(city)
    local multiplier = needed and (1.0 + rain * 0.35 + fog * 0.30 + night * 0.28) or 0.32
    local intensity = base * multiplier
    if self.entity.tag == "Police" then
        intensity = (alert and (2.0 + wanted * 0.28) or 1.35) * (needed and 1.15 or 0.72)
        Component.set(self.entity, "Light2D", "color", alert and { 105, 165, 255 } or { 135, 185, 255 })
        Component.set(self.entity, "Light2D", "flicker", alert)
    else
        Component.set(self.entity, "Light2D", "color", needed and { 255, 224, 166 } or { 154, 128, 92 })
        Component.set(self.entity, "Light2D", "flicker", rain > 0.7 or fog > 0.55)
    end
    Component.set(self.entity, "Light2D", "radius", needed and (3.0 + fog * 1.8 + night * 1.0) or 1.7)
    Component.set(self.entity, "Light2D", "intensity", intensity)
end

local function blocker_exists(self)
    local blockers = Entity.nearby(self.entity, 1.35, { tags = { "Player", "Police" } })
    for _, blocker in ipairs(blockers) do
        if blocker.id ~= self.entity_id then
            return true
        end
    end
    return false
end

local function normalized_axis()
    local x = Input.axis("move_left", "move_right")
    local y = Input.axis("move_up", "move_down")
    local length = math.sqrt(x * x + y * y)
    if length > 0.01 then
        return x / length, y / length, true
    end
    return 0, 0, false
end

function Traffic:drive_player_vehicle(dt)
    local city = city_state()
    local x, y, moving = normalized_axis()
    local boost = Input.action_pressed("run") and 1.28 or 1.0
    local road_factor = city.traffic_factor or 1.0
    if moving then
        Rigidbody2D.set_velocity(self.entity, x * self.drive_speed * boost * road_factor, y * self.drive_speed * boost * road_factor)
        Component.set(self.entity, "Transform", "rotation", math.deg(math.atan(y, x)))
        Component.set(self.entity, "SpriteRenderer", "tint", self.entity.tag == "Police" and { 112, 170, 255 } or { 255, 214, 136 })
    else
        Rigidbody2D.set_velocity(self.entity, 0, 0)
    end
    Component.set(self.entity, "NavAgent", "has_destination", false)
    Component.set(self.entity, "NavAgent", "speed", 0.0)
    self:apply_lighting(city, self.entity.tag == "Police" and 2.0 or 1.35, self.entity.tag == "Police")
    if Input.action_pressed("fire") and self.horn_timer <= 0 then
        self.horn_timer = 1.2
        Audio2D.play(self.entity.tag == "Police" and "police_siren_burst" or "traffic_horn", { volume = 0.42, bus = "SFX" })
    end
    Component.set(self.entity, "Blackboard", "values", {
        route = self.route,
        route_index = self.route_index,
        speed = values(self.entity).speed or 4.2,
        drive_speed = self.drive_speed,
        display_name = self.display_name,
        drivable = true,
        occupied = true,
        driver = self.driver,
        vehicle_class = self.entity.tag == "Police" and "police" or "civilian",
    })
end

function Traffic:drive_police_ai(dt)
    local city = city_state()
    local wanted = city.wanted or 0
    if self.entity.tag ~= "Police" or wanted < 1.0 then
        return false
    end

    local player = Entity.find("Player")
    if not player then
        return false
    end
    local me = self.entity.transform.position
    local player_pos = player.transform.position
    local dx = player_pos.x - me.x
    local dy = player_pos.y - me.y
    local distance = math.sqrt(dx * dx + dy * dy)
    local sight_range = (11.0 + wanted * 3.0) * math.max(0.62, 1.0 - (city.fog or 0) * 0.28 - (city.rain or 0) * 0.10)
    local blocked = Physics2D.raycast(me, player_pos, { mask = Layers.WORLD_STATIC, include_triggers = false })
    local can_see = distance <= sight_range and not blocked

    if can_see then
        self.last_seen_x = player_pos.x
        self.last_seen_y = player_pos.y
        self.state = "pursuit"
        Component.set(self.entity, "NavAgent", "speed", 5.5 + wanted * 0.9)
        Navigation2D.set_destination(self.entity, player_pos.x, player_pos.y)
        self:apply_lighting(city, 2.1 + wanted * 0.22, true)
        Component.set(self.entity, "SpriteRenderer", "show_label", true)
        if distance < 1.45 and wanted >= 2.0 then
            Events.emit("player_arrested", { officer = self.entity.name, method = "vehicle_box" })
        end
        return true
    elseif self.last_seen_x and wanted >= 1.5 then
        self.state = "search"
        Component.set(self.entity, "NavAgent", "speed", 4.2 + wanted * 0.45)
        Navigation2D.set_destination(self.entity, self.last_seen_x, self.last_seen_y)
        self:apply_lighting(city, 1.6 + wanted * 0.15, true)
        return true
    end

    return false
end

function Traffic:on_update(dt)
    self.rethink = self.rethink - dt
    self.horn_timer = math.max(0, self.horn_timer - dt)
    self.resume_timer = math.max(0, self.resume_timer - dt)

    if self.occupied then
        self:drive_player_vehicle(dt)
        return
    end

    if self.rethink > 0 then
        return
    end
    self.rethink = 1.0

    local city = city_state()
    if self:drive_police_ai(dt) then
        Component.set(self.entity, "Blackboard", "values", {
            route = self.route,
            route_index = self.route_index,
            speed = values(self.entity).speed or 5.2,
            drive_speed = self.drive_speed,
            display_name = self.display_name,
            drivable = true,
            occupied = false,
            vehicle_class = "police",
            state = self.state,
            last_seen_x = self.last_seen_x,
            last_seen_y = self.last_seen_y,
        })
        return
    end

    if self.resume_timer > 0 or blocker_exists(self) then
        Component.set(self.entity, "NavAgent", "speed", 0.0)
        if self.horn_timer <= 0 then
            self.horn_timer = 4.0
            Audio2D.play("traffic_horn", { volume = 0.35, bus = "SFX" })
        end
        return
    end

    local data = values(self.entity)
    self.state = "route"
    Component.set(self.entity, "NavAgent", "speed", (data.speed or 4.2) * (city.traffic_factor or 1.0))
    self:apply_lighting(city, self.entity.tag == "Police" and 1.4 or 0.75, false)
    if #self.route > 0 then
        local target = self.route[self.route_index]
        Navigation2D.set_destination(self.entity, target.x, target.y)
        self.route_index = self.route_index + 1
        if self.route_index > #self.route then
            self.route_index = 1
        end
    end
end

function Traffic:on_event(name, payload)
    if name == "vehicle_entered" and payload.vehicle_id == self.entity_id then
        self.occupied = true
        self.driver = payload.player_id or "Player"
        self.resume_timer = 0
        Rigidbody2D.set_velocity(self.entity, 0, 0)
        Component.set(self.entity, "SpriteRenderer", "show_label", true)
    elseif name == "vehicle_exited" and payload.vehicle_id == self.entity_id then
        self.occupied = false
        self.driver = nil
        self.resume_timer = 1.2
        Rigidbody2D.set_velocity(self.entity, 0, 0)
        Component.set(self.entity, "SpriteRenderer", "show_label", self.entity.tag == "Police")
        self:apply_lighting(city_state(), self.entity.tag == "Police" and 1.4 or 0.75, false)
    elseif name == "player_arrested" and self.occupied then
        self.occupied = false
        self.driver = nil
        self.resume_timer = 2.0
        Rigidbody2D.set_velocity(self.entity, 0, 0)
    elseif name == "crime_committed" and self.entity.tag == "Police" then
        self.last_seen_x = payload.x
        self.last_seen_y = payload.y
        self.rethink = 0
    elseif name == "wanted_changed" and (payload.wanted or 0) <= 0 then
        self.last_seen_x = nil
        self.last_seen_y = nil
        self.state = "route"
    end
end

return Traffic
