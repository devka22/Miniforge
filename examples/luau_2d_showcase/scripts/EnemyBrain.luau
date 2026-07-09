local Enemy = {
    direction = -1,
}

function Enemy:on_start()
    AnimationPlayer.play(self.entity, "Idle")
    Component.set(self.entity, "Collider2D", "collision_layer", Layers.ENEMY)
end

function Enemy:on_update(dt)
    local origin = self.entity.transform.position
    local probe = Vector2.new(origin.x + self.direction * 1.2, origin.y)
    local wall = Physics2D.raycast(origin, probe, { mask = Layers.WORLD_STATIC })
    if wall then
        self.direction = -self.direction
    end
    Rigidbody2D.set_velocity(self.entity, self.direction * 1.5, 0.0)
end

function Enemy:on_collision_enter(other)
    if other == "Projectile" then
        Events.emit("enemy_defeated", { score = 100 })
        Audio2D.play("enemy_hit", { volume = 0.8 })
        destroy()
    end
end

function Enemy:on_event(name, payload)
    if name == "physics_trigger_enter" and payload.other_name == "Player" then
        AnimationPlayer.set_parameter(self.entity, "alert", true)
    end
end

return Enemy
