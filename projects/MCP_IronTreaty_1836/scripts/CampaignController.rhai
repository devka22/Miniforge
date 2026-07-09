fn on_start() {
    set_ui_text("HUD_Campaign", "Iron Treaty 1836 listo: economia, pops, diplomacia e industria cargadas.");
}

fn on_update(dt) {
    if input_pressed("Space") {
        set_ui_text("HUD_Date", "Simulacion mensual en pausa tactica");
    }
}
