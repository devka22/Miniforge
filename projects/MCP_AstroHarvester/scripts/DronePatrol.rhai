fn on_start() {
    ui_text("Drone activo");
}

fn on_update(dt) {
    move(-0.35 * dt, 0.0);
}
