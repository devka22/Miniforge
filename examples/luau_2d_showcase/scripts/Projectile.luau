local Projectile = {}

function Projectile:on_start()
    Task.delay(1.2, function()
        destroy()
    end)
end

function Projectile:on_collision_enter(other)
    if other ~= "Player" then
        destroy()
    end
end

return Projectile
