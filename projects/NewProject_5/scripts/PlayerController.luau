local CityDB = require("./CityDB")

local Player = {
    fire_cooldown = 0,
    interact_cooldown = 0,
    pickup_cooldown = 0,
    wanted = 0,
    cash = 120,
    heat_memory = 0,
    facing_x = 1,
    facing_y = 0,
    vehicle_id = nil,
    vehicle_name = nil,
    in_vehicle = false,
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

local function set_player_hud(self)
    set_ui_text("HUD_Wanted", "Busqueda " .. tostring(math.floor(self.wanted + 0.5)))
    local cash = tostring(math.floor(self.cash))
    while string.len(cash) < 6 do
        cash = "0" .. cash
    end
    set_ui_text("HUD_Cash", "$" .. cash)
    set_ui_text("HUD_Heat", CityDB.dispatch_label(self.wanted))
    Component.set("HUD_Health", "UIElement", "progress", 100.0)
    Component.set("HUD_Armor", "UIElement", "progress", 100.0)
end

local function set_prompt(text)
    set_ui_text("HUD_Prompt", text)
    if text == nil or text == "" then
        Component.set("HUD_Prompt", "UIElement", "opacity", 0.0)
    else
        Component.set("HUD_Prompt", "UIElement", "opacity", 0.86)
    end
end

local function drivable_nearby(self)
    local nearby = Entity.nearby(self.entity, 2.15, { tags = { "Vehicle", "Police" } })
    for _, candidate in ipairs(nearby) do
        local data = Component.get(candidate, "Blackboard", "values", {})
        if data.drivable then
            return candidate, data
        end
    end
    return nil, nil
end

function Player:enter_vehicle(vehicle, data)
    self.in_vehicle = true
    self.vehicle_id = vehicle.id
    self.vehicle_name = vehicle.name
    set_visible(false)
    Component.set(self.entity, "Collider2D", "is_trigger", true)
    Component.set(self.entity, "Collider2D", "collision_mask", {})
    Component.set(self.entity, "CharacterController2D", "input_enabled", false)
    Camera.main():follow(vehicle)
    set_prompt("F: salir de " .. (data.display_name or vehicle.name))
    set_blackboard("current_vehicle", vehicle.name)
    Events.emit("vehicle_entered", {
        vehicle_id = vehicle.id,
        vehicle_name = vehicle.name,
        vehicle_tag = vehicle.tag,
        player_id = self.entity_id,
    })
    local boost = vehicle.tag == "Police" and 1.6 or 0.45
    self.wanted = clamp(self.wanted + boost, 0, 5)
    self.heat_memory = 6.0
    set_blackboard("wanted", self.wanted)
    set_player_hud(self)
    Events.emit("crime_committed", {
        kind = vehicle.tag == "Police" and "police_vehicle_theft" or "vehicle_theft",
        wanted = self.wanted,
        x = vehicle.transform.position.x,
        y = vehicle.transform.position.y,
    })
end

function Player:exit_vehicle()
    local vehicle = self.vehicle_id and Entity.find({ id = self.vehicle_id }) or nil
    local x = self.entity.transform.position.x
    local y = self.entity.transform.position.y
    if vehicle then
        x = vehicle.transform.position.x + 1.25
        y = vehicle.transform.position.y + 0.25
        Events.emit("vehicle_exited", {
            vehicle_id = self.vehicle_id,
            vehicle_name = self.vehicle_name,
        })
    end
    self.in_vehicle = false
    self.vehicle_id = nil
    self.vehicle_name = nil
    Transform2D.set_position(self.entity, x, y)
    set_visible(true)
    Component.set(self.entity, "Collider2D", "is_trigger", false)
    Component.set(self.entity, "Collider2D", "collision_mask", { "WorldStatic", "NPC", "Police", "Vehicle", "Trigger", "Collectible" })
    Component.set(self.entity, "CharacterController2D", "input_enabled", true)
    Camera.main():follow(self.entity)
    set_blackboard("current_vehicle", nil)
    set_prompt("")
end

function Player:on_start()
    self.speed_walk = 6.0
    self.speed_run = 8.75
    Camera.main():follow(self.entity)
    Camera.main():set_zoom(1.45)
    Camera.main():pixel_perfect(true, 16)
    add_quest(
        "act1_rain_on_the_bridge",
        "Acto I: Lluvia sobre el puente",
        "meet_mara",
        "Habla con Mara en Bajo Muelle.",
        1
    )
    set_blackboard("wanted", self.wanted)
    set_blackboard("cash", self.cash)
    set_player_hud(self)
    Events.emit("player_ready", { city = CityDB.world.city })
end

function Player:on_update(dt)
    self.fire_cooldown = math.max(0, self.fire_cooldown - dt)
    self.interact_cooldown = math.max(0, self.interact_cooldown - dt)
    self.pickup_cooldown = math.max(0, self.pickup_cooldown - dt)

    if self.in_vehicle then
        local vehicle = self.vehicle_id and Entity.find({ id = self.vehicle_id }) or nil
        if vehicle then
            Transform2D.set_position(self.entity, vehicle.transform.position.x, vehicle.transform.position.y)
            CharacterBody2D.move(self.entity, 0, 0, false, false)
            if Input.action_pressed("enter_vehicle") and self.interact_cooldown <= 0 then
                self.interact_cooldown = 0.45
                self:exit_vehicle()
            else
                set_prompt("F: salir de " .. (self.vehicle_name or "vehiculo"))
            end
        else
            self:exit_vehicle()
        end
        if self.heat_memory > 0 then
            self.heat_memory = math.max(0, self.heat_memory - dt)
        end
        return
    end

    local x = Input.axis("move_left", "move_right")
    local y = Input.axis("move_up", "move_down")
    local moving = math.abs(x) + math.abs(y) > 0.01
    local running = Input.action_pressed("run")
    local speed = running and self.speed_run or self.speed_walk

    if moving then
        local length = math.sqrt(x * x + y * y)
        if length > 0 then
            x = x / length
            y = y / length
        end
        self.facing_x = x
        self.facing_y = y
        AnimationPlayer.play(self.entity, running and "Run" or "Walk")
        Component.set(self.entity, "CharacterController2D", "walk_speed", speed)
        Component.set(self.entity, "SpriteRenderer", "flip_x", x < -0.05)
    else
        AnimationPlayer.play(self.entity, "Idle")
    end

    CharacterBody2D.move(self.entity, x, y, false, running)

    local prompt = ""
    local contacts = Entity.nearby(self.entity, 1.85, { tag = "Contact" })
    if #contacts > 0 then
        prompt = "E: hablar con " .. contacts[1].name
    else
        local pickups = Entity.nearby(self.entity, 1.25, { tag = "Collectible" })
        if #pickups > 0 then
            prompt = "E: recoger " .. pickups[1].name
        else
            local vehicle, data = drivable_nearby(self)
            if vehicle then
                prompt = "F: conducir " .. (data.display_name or vehicle.name)
            end
        end
    end
    set_prompt(prompt)

    if Input.action_pressed("fire") and self.fire_cooldown <= 0 then
        self.fire_cooldown = 0.34
        self.wanted = clamp(self.wanted + 0.85, 0, 5)
        self.heat_memory = 5.0
        set_blackboard("wanted", self.wanted)
        set_player_hud(self)
        local origin = self.entity.transform.position
        local shot_x = self.facing_x
        local shot_y = self.facing_y
        if math.abs(shot_x) + math.abs(shot_y) < 0.01 then
            shot_x = 1
            shot_y = 0
        end
        Entity.spawn("TracerRound", origin.x + shot_x * 0.8, origin.y + shot_y * 0.8, {
            tag = "Projectile",
            layer = "Projectile",
            script = "PickupBrain.luau",
            width = 0.25,
            height = 0.25,
            components = {
                { component_type = "Rigidbody2D", body_type = "kinematic", velocity_x = shot_x * 15.0, velocity_y = shot_y * 15.0, use_gravity = false },
                { component_type = "Collider2D", shape = "rect", width = 0.25, height = 0.25, collision_layer = "Projectile", collision_mask = { "WorldStatic", "Enemy" } },
                { component_type = "Lifetime", duration = 0.75, destroy_on_expire = true },
                { component_type = "Light2D", light_type = "point", radius = 2.6, intensity = 1.8, color = { 255, 205, 120 }, casts_shadows = true }
            }
        })
        Audio2D.play("pistol_dry_neon", { volume = 0.7, bus = "SFX" })
        Camera.main():shake(0.18, 4.0)
        Events.emit("crime_committed", {
            kind = "weapon_discharge",
            wanted = self.wanted,
            x = origin.x,
            y = origin.y,
        })
    end

    if self.heat_memory > 0 then
        self.heat_memory = math.max(0, self.heat_memory - dt)
    elseif self.wanted > 0 then
        self.wanted = math.max(0, self.wanted - 0.018 * dt)
        set_blackboard("wanted", self.wanted)
        set_player_hud(self)
    end

    if Input.action_pressed("enter_vehicle") and self.interact_cooldown <= 0 then
        local vehicle, data = drivable_nearby(self)
        if vehicle then
            self.interact_cooldown = 0.45
            self:enter_vehicle(vehicle, data)
            return
        end
    end

    if Input.action_pressed("interact") and self.interact_cooldown <= 0 then
        self.interact_cooldown = 0.35
        local contacts = Entity.nearby(self.entity, 1.85, { tag = "Contact" })
        if #contacts > 0 then
            Events.emit("contact_interact", {
                contact_name = contacts[1].name,
                distance = contacts[1].distance,
            })
        else
            local pickups = Entity.nearby(self.entity, 1.25, { tag = "Collectible" })
            if #pickups > 0 then
                Events.emit("pickup_collected", {
                    pickup_name = pickups[1].name,
                    value = Component.get(pickups[1], "Blackboard", "values", {}).value or 25,
                })
                Entity.destroy(pickups[1])
            end
        end
    end
end

function Player:on_event(name, payload)
    if name == "wanted_changed" then
        self.wanted = clamp(payload.wanted or self.wanted, 0, 5)
        set_blackboard("wanted", self.wanted)
        set_player_hud(self)
    elseif name == "mission_completed" then
        complete_quest(payload.id or "act1_rain_on_the_bridge")
        self.cash = self.cash + (payload.reward or 150)
        set_blackboard("cash", self.cash)
        set_player_hud(self)
        Audio2D.play("mission_complete", { volume = 0.8, bus = "UI" })
    elseif name == "player_arrested" then
        if self.in_vehicle then
            self:exit_vehicle()
        end
        self.wanted = 0
        self.heat_memory = 0
        set_blackboard("wanted", 0)
        set_player_hud(self)
        Camera.main():shake(0.5, 8.0)
    end
end

function Player:on_collision_enter(other)
    if other == "PoliceCar" or other == "PatrolOfficer" then
        Camera.main():shake(0.22, 4.0)
    end
end

return Player
