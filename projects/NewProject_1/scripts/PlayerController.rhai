// MiniForge Rhai script: PlayerController
// Attach this file to an entity with script = "PlayerController.rhai".

fn on_start() {
    // play_sound("spawn");
}

fn on_update(dt) {
    if input_pressed("A") {
        move(-4.0 * dt, 0.0);
    }
    if input_pressed("D") {
        move(4.0 * dt, 0.0);
    }
}

fn on_key_down(key) {
    if key == "Space" {
        play_sound("jump");
    }
}

fn on_collision_enter(other) {
    // ui_text("Hit " + other);
}

fn on_destroy() {
}
