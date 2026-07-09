local Player = {
    fire_cooldown = 0,
    score = 0,
}

function Player:on_start()
    Camera.main():follow(self.entity)
    Camera.main():set_zoom(2.0)
    Camera.main():pixel_perfect(true, 16)

    Component.add(self.entity, "ParticleEmitter", {
        component_type = "ParticleEmitter",
        looped = false,
        burst_count = 0,
        lifetime = 0.35,
        velocity_y = -24.0,
        max_particles = 64,
    })

    Events.emit("showcase_started", { hot_reload = true })

    Task.delay(1.5, function()
        Debug.log("Task.delay fired; edit and save this file to test hot reload")
    end)
end

function Player:on_update(dt)
    local move_x = Input.axis("move_left", "move_right")
    local jump = Input.action_pressed("jump")
    local run = Input.action_pressed("run")

    CharacterBody2D.move(self.entity, move_x, 0.0, jump, run)

    if move_x ~= 0 then
        AnimationPlayer.play(self.entity, "Run")
        Component.set(self.entity, "SpriteRenderer", "flip_x", move_x < 0)
    else
        AnimationPlayer.play(self.entity, "Idle")
    end

    self.fire_cooldown = math.max(0, self.fire_cooldown - dt)
    if Input.action_pressed("fire") and self.fire_cooldown <= 0 then
        self.fire_cooldown = 0.25
        local facing = 1
        if move_x < 0 then facing = -1 end
        Entity.spawn("Projectile", self.entity.transform.position.x + facing * 0.8, self.entity.transform.position.y, {
            tag = "Projectile",
            layer = "Projectile",
            script = "Projectile.luau",
            width = 0.25,
            height = 0.25,
            components = {
                { component_type = "Rigidbody2D", body_type = "kinematic", velocity_x = facing * 14.0, velocity_y = 0.0, use_gravity = false },
                { component_type = "Collider2D", shape = "rect", width = 0.25, height = 0.25, collision_layer = "Projectile", collision_mask = {"Enemy", "WorldStatic"} },
                { component_type = "Lifetime", duration = 1.2, destroy_on_expire = true },
                { component_type = "DamageDealer", damage = 25.0, target_tags = {"Enemy"}, hit_once = true }
            }
        })
        Audio2D.play("shoot", { volume = 0.65 })
        Particles2D.burst(self.entity, 8)
    end

    local origin = self.entity.transform.position
    local target = Vector2.new(origin.x + 6.0, origin.y)
    local hit = Physics2D.raycast(origin, target, { mask = Layers.ENEMY, include_triggers = true })
    if hit then
        AnimationPlayer.set_parameter(self.entity, "enemy_in_sight", true)
    else
        AnimationPlayer.set_parameter(self.entity, "enemy_in_sight", false)
    end

    local ground = Tilemap.get_tile("ShowcaseTilemap", "Collision", 4, 12)
    if ground == 0 then
        Tilemap.set_tile("ShowcaseTilemap", "Collision", 4, 12, 1)
    end
end

function Player:on_collision_enter(other)
    Camera.main():shake(0.3, 6.0)
    Events.emit("player_collision", { other = other })
end

function Player:on_event(name, payload)
    if name == "enemy_defeated" then
        self.score = self.score + (payload.score or 0)
        Component.set("HudScore", "UIElement", "text", "Score " .. tostring(self.score))
        Tween.to(self.entity, "scale_x", 1.15, 0.08, { easing = "ease_out" })
    end
end

return Player
