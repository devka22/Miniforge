// MiniForge Rhai script: NewGameplayScript
// Attach this file to an entity with script = "NewGameplayScript.rhai".

fn on_start() {
    // play_sound("spawn");
    // set_sprite("assets/sprites/player.sprite.json");
    // play_sprite_animation("assets/animations/player.spriteframes", "default");
}

fn on_update(dt) {
    if input_pressed("A") {
        face_left();
        move(-4.0 * dt, 0.0);
    }
    if input_pressed("D") {
        face_right();
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
