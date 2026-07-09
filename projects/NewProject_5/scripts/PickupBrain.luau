local Pickup = {}

function Pickup:on_start()
    self.life = Component.get(self.entity, "Lifetime", "duration", -1)
    self.spin = 0
end

function Pickup:on_update(dt)
    self.spin = self.spin + dt
    Component.set(self.entity, "SpriteRenderer", "tint", {
        220 + math.floor(math.sin(self.spin * 5.0) * 25),
        235,
        255,
    })
    if self.entity.tag == "Projectile" then
        return
    end
    local player = Entity.nearby(self.entity, 0.9, { tag = "Player" })
    if #player > 0 then
        local data = Component.get(self.entity, "Blackboard", "values", {})
        Events.emit("pickup_collected", {
            pickup_name = self.entity.name,
            value = data.value or 25,
        })
        destroy()
    end
end

function Pickup:on_collision_enter(other)
    if self.entity.tag == "Projectile" and other ~= "Player" then
        destroy()
    end
end

return Pickup
