local CityDB = require("./CityDB")

local Pedestrian = {
    route_index = 1,
    rethink = 0,
    panic = 0,
    gossip_timer = 0,
    witness_timer = 0,
    pause_timer = 0,
    mood = "wander",
    last_line = "",
}

local function values(entity)
    return Component.get(entity, "Blackboard", "values", {})
end

local function distance(a, b)
    local dx = a.x - b.x
    local dy = a.y - b.y
    return math.sqrt(dx * dx + dy * dy)
end

function Pedestrian:on_start()
    local data = values(self.entity)
    self.route = data.route or {}
    self.persona = data.persona or "vecino"
    self.routine = data.routine or self.persona
    self.route_index = data.route_index or 1
    local district = CityDB.district_at(self.entity.transform.position.x, self.entity.transform.position.y)
    self.district = data.district or district.name
    self.gossip_timer = 1.0 + ((self.entity_id or 1) % 6) * 0.35
    self.witness_timer = ((self.entity_id or 1) % 5) * 0.5
    self.pause_timer = ((self.entity_id or 1) % 4) * 0.25
    Component.set(self.entity, "NavAgent", "speed", data.speed or 2.2)
    AnimationPlayer.play(self.entity, "Walk")
end

function Pedestrian:publish()
    Component.set(self.entity, "Blackboard", "values", {
        persona = self.persona,
        routine = self.routine,
        route = self.route,
        route_index = self.route_index,
        speed = values(self.entity).speed or 2.2,
        district = self.district,
        mood = self.mood,
        panic = self.panic,
        last_line = self.last_line,
    })
end

function Pedestrian:on_update(dt)
    self.rethink = self.rethink - dt
    self.panic = math.max(0, self.panic - dt)
    self.gossip_timer = math.max(0, self.gossip_timer - dt)
    self.witness_timer = math.max(0, self.witness_timer - dt)
    self.pause_timer = math.max(0, self.pause_timer - dt)
    local city = values("CityDirector")
    local wanted = city.wanted or 0
    local mission_index = city.mission_index or 1
    local rain = city.rain or 0
    local fog = city.fog or 0
    local night = city.night_factor or 0
    local player = Entity.find("Player")
    local player_distance = 999

    local base_speed = values(self.entity).speed or 2.2
    local weather_factor = 1.0 - rain * 0.18 - fog * 0.08
    if self.routine == "jog" and wanted < 1.5 and rain < 0.4 then
        weather_factor = weather_factor + 0.22
    elseif self.routine == "tour" and self.district == "Playa Neon" and night > 0.38 then
        weather_factor = weather_factor + 0.12
    elseif self.routine == "watch" or self.routine == "sell" then
        weather_factor = weather_factor - 0.10
    end
    Component.set(self.entity, "NavAgent", "speed", math.max(1.15, base_speed * weather_factor))

    if self.rethink <= 0 then
        self.rethink = self.panic > 0 and 0.45 or (2.0 + ((self.entity_id or 1) % 5) * 0.22)
        if player and self.panic > 0 then
            local me = self.entity.transform.position
            local p = player.transform.position
            local dx = me.x - p.x
            local dy = me.y - p.y
            local len = math.max(0.1, math.sqrt(dx * dx + dy * dy))
            self.mood = "panic"
            Navigation2D.set_destination(self.entity, me.x + dx / len * 5.5, me.y + dy / len * 5.5)
            self.last_line = CityDB.pedestrian_line(wanted, mission_index, self.persona, true, self.district)
            set_ui_text("HUD_Radio", self.last_line)
            self:publish()
        elseif self.pause_timer > 0 and (self.routine == "sell" or self.routine == "watch" or self.routine == "perform") then
            self.mood = self.routine
            Component.set(self.entity, "NavAgent", "speed", 0.0)
            self:publish()
        elseif #self.route > 0 then
            local target = self.route[self.route_index]
            if wanted >= 2.0 then
                self.mood = "uneasy"
            elseif rain > 0.55 then
                self.mood = "hurry"
            elseif self.routine == "tour" and self.district == "Playa Neon" then
                self.mood = "tour"
            elseif self.routine == "perform" and night > 0.32 then
                self.mood = "party"
            else
                self.mood = "wander"
            end
            Navigation2D.set_destination(self.entity, target.x, target.y)
            self.route_index = self.route_index + 1
            if self.route_index > #self.route then
                self.route_index = 1
            end
            self.pause_timer = ((self.entity_id or 1) % 3 == 0) and (1.0 + ((self.entity_id or 1) % 4) * 0.35) or 0
            self:publish()
        end
    end

    if player then
        player_distance = distance(self.entity.transform.position, player.transform.position)
    end
    if player and player_distance < 4.2 and self.panic <= 0 and self.gossip_timer <= 0 then
        self.gossip_timer = 7.5 + ((self.entity_id or 1) % 7)
        self.last_line = CityDB.pedestrian_line(wanted, mission_index, self.persona, false, self.district)
        if rain > 0.45 then
            self.last_line = "La lluvia vuelve brillante todo lo que deberia esconderse."
        elseif self.district == "Playa Neon" and night > 0.35 then
            self.last_line = CityDB.pedestrian_line(wanted, mission_index, self.persona, false, "Playa Neon")
        end
        self.mood = wanted >= 2.0 and "uneasy" or "comment"
        set_ui_text("HUD_Radio", self.persona .. ": " .. self.last_line)
        self:publish()
    end
    if player and wanted >= 2.4 and player_distance < 6.0 and self.witness_timer <= 0 then
        self.witness_timer = 14.0 + ((self.entity_id or 1) % 5)
        self.panic = math.max(self.panic, 3.5)
        self.mood = "witness"
        Events.emit("witness_reported", {
            npc = self.entity.name,
            district = self.district,
            confidence = math.min(1.0, wanted / 5.0),
            x = player.transform.position.x,
            y = player.transform.position.y,
        })
        self:publish()
    end
end

function Pedestrian:on_event(name, payload)
    if name == "crime_committed" or name == "wanted_changed" then
        local player = Entity.find("Player")
        if player and distance(self.entity.transform.position, player.transform.position) < 8.0 then
            self.panic = 5.0
            self.mood = "panic"
            self.last_line = CityDB.pedestrian_line(payload.wanted or 2.0, 1, self.persona, true, self.district)
            Component.set(self.entity, "SpriteRenderer", "tint", { 255, 210, 195 })
            self:publish()
        end
    elseif name == "player_arrested" then
        self.panic = 0
        self.mood = "wander"
        Component.set(self.entity, "SpriteRenderer", "tint", { 230, 235, 245 })
        self:publish()
    end
end

return Pedestrian
