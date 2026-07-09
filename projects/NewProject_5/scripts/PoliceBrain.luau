local Police = {
    route_index = 1,
    state = "patrol",
    rethink = 0,
    arrest_cooldown = 0,
    last_seen_x = nil,
    last_seen_y = nil,
}

local function values(entity)
    return Component.get(entity, "Blackboard", "values", {})
end

local function dist(a, b)
    local dx = a.x - b.x
    local dy = a.y - b.y
    return math.sqrt(dx * dx + dy * dy)
end

local function apply_light(entity, city, wanted, state)
    local fog = city.fog or 0
    local rain = city.rain or 0
    local night = city.night_factor or 0
    local alert = state == "chase" or state == "search"
    Component.set(entity, "Light2D", "color", alert and { 120, 170, 255 } or { 150, 190, 255 })
    Component.set(entity, "Light2D", "radius", 4.4 + night * 2.0 + fog * 1.6)
    Component.set(entity, "Light2D", "intensity", (alert and (1.65 + wanted * 0.22) or 1.05) + night * 0.35 + rain * 0.12)
    Component.set(entity, "Light2D", "flicker", alert and wanted >= 2.0)
end

function Police:on_start()
    local data = values(self.entity)
    self.route = data.route or {}
    self.route_index = data.route_index or 1
    Component.set(self.entity, "NavAgent", "speed", data.speed or 3.4)
    Component.set(self.entity, "AIController", "behavior", "guard")
    Component.set(self.entity, "AIController", "target_tags", { "Player" })
    AnimationPlayer.play(self.entity, "Patrol")
end

function Police:on_update(dt)
    self.rethink = self.rethink - dt
    self.arrest_cooldown = math.max(0, self.arrest_cooldown - dt)
    if self.rethink > 0 then
        return
    end
    self.rethink = 0.28

    local city = values("CityDirector")
    local wanted = city.wanted or 0
    local fog = city.fog or 0
    local rain = city.rain or 0
    local player = Entity.find("Player")
    if not player then
        return
    end

    local me = self.entity.transform.position
    local player_pos = player.transform.position
    local distance_to_player = dist(me, player_pos)
    local sight_range = (7.0 + wanted * 2.2) * math.max(0.58, 1.0 - fog * 0.30 - rain * 0.12)
    local blocked = Physics2D.raycast(me, player_pos, { mask = Layers.WORLD_STATIC, include_triggers = false })
    local can_see = wanted > 0.1 and distance_to_player <= sight_range and not blocked

    if can_see then
        self.state = "chase"
        self.last_seen_x = player_pos.x
        self.last_seen_y = player_pos.y
        Component.set(self.entity, "NavAgent", "speed", 4.0 + wanted * 0.35)
        Navigation2D.set_destination(self.entity, player_pos.x, player_pos.y)
        apply_light(self.entity, city, wanted, self.state)
        if distance_to_player < 1.05 and self.arrest_cooldown <= 0 and wanted >= 2.0 then
            self.arrest_cooldown = 4.0
            Events.emit("player_arrested", { officer = self.entity.name })
        end
    elseif wanted > 0.1 and self.last_seen_x then
        self.state = "search"
        Navigation2D.set_destination(self.entity, self.last_seen_x, self.last_seen_y)
        apply_light(self.entity, city, wanted, self.state)
    elseif #self.route > 0 then
        self.state = "patrol"
        Component.set(self.entity, "NavAgent", "speed", 3.0)
        local target = self.route[self.route_index]
        Navigation2D.set_destination(self.entity, target.x, target.y)
        self.route_index = self.route_index + 1
        if self.route_index > #self.route then
            self.route_index = 1
        end
        apply_light(self.entity, city, wanted, self.state)
    end

    Component.set(self.entity, "Blackboard", "values", {
        state = self.state,
        wanted_seen = wanted,
        last_seen_x = self.last_seen_x,
        last_seen_y = self.last_seen_y,
        route = self.route,
        route_index = self.route_index,
    })
end

function Police:on_event(name, payload)
    if name == "crime_committed" then
        self.last_seen_x = payload.x
        self.last_seen_y = payload.y
        self.rethink = 0
    elseif name == "wanted_changed" and (payload.wanted or 0) <= 0 then
        self.last_seen_x = nil
        self.last_seen_y = nil
        self.state = "patrol"
    end
end

return Police
