fn on_start() {
    set_ui_text("HUD_Status", "Pilot online. WASD mueve, Space dispara pulso.");
}

fn on_update(dt) {
    if input_pressed("A") { move(-5.0 * dt, 0.0); }
    if input_pressed("D") { move(5.0 * dt, 0.0); }
    if input_pressed("W") { move(0.0, -5.0 * dt); }
    if input_pressed("S") { move(0.0, 5.0 * dt); }
    if input_pressed("Space") {
        set_ui_text("HUD_Status", "Pulso de defensa emitido. Revisa enemigos cercanos.");
    }
}

fn on_collision_enter(other) {
    set_ui_text("HUD_Status", "Contacto detectado: " + other);
}
