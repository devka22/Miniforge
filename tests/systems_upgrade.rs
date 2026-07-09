use serde_json::json;

use miniforge::engine::animation_graph::{AnimationEvent, AnimationGraphLibrary};
use miniforge::engine::audio_mixer::AudioMixer;
use miniforge::engine::camera::Camera;
use miniforge::engine::component::default_component;
use miniforge::entities::game_object::GameObject;
use miniforge::systems::animation_system::AnimationSystem;
use miniforge::systems::audio_system::AudioSystem;
use miniforge::systems::camera_system::CameraSystem;
use miniforge::systems::editor_system::EditorSystem;
use miniforge::systems::input_system::InputSystem;
use miniforge::systems::movement_system::MovementSystem;
use miniforge::systems::physics_system::PhysicsSystem;
use miniforge::systems::render_system::RenderSystem;
use miniforge::systems::ui_system::UISystem;

#[test]
fn movement_skips_disabled_entities_and_sanitizes_bad_state() {
    let mut disabled = GameObject::new_unit(0.0, 0.0, Some("Disabled".to_string()));
    disabled.enabled = false;
    disabled.path = vec![(10.0, 0.0)];
    let mut invalid = GameObject::new_unit(f64::NAN, 0.0, Some("Invalid".to_string()));
    invalid.path = vec![(1.0, 0.0)];
    let mut entities = vec![disabled, invalid];

    let report = MovementSystem.update_entities_with_report(&mut entities, 0.016);

    assert_eq!(entities[0].x, 0.0);
    assert!(entities[1].x.is_finite());
    assert_eq!(report.skipped, 1);
    assert_eq!(report.sanitized, 1);
}

#[test]
fn input_queue_preserves_frame_edges() {
    let mut input = InputSystem::default();
    input.queue_key("Space", true);
    input.queue_pointer(12.0, 34.0);
    input.update();

    assert!(input.handler.state.just_pressed("space"));
    assert_eq!(input.handler.state.mouse_position, (12.0, 34.0));
    assert_eq!(input.stats["events"], 2);

    input.update();
    assert!(!input.handler.state.just_pressed("space"));
}

#[test]
fn editor_and_ui_systems_are_functional_facades() {
    let mut first = GameObject::new(0.0, 0.0, Some("Button".to_string()));
    let mut ui = default_component("UIElement").unwrap();
    ui.set("element_type", json!("Button"));
    ui.set("interactable", json!(true));
    ui.set_f64("x", 10.0);
    ui.set_f64("y", 10.0);
    ui.set_f64("width", 100.0);
    ui.set_f64("height", 40.0);
    first.add_component(ui);
    let mut duplicate = GameObject::new(20.0, 20.0, Some("Duplicate".to_string()));
    duplicate.id = first.id;
    let mut entities = vec![first, duplicate];

    let mut editor = EditorSystem::default();
    assert_eq!(editor.update(&entities).duplicate_ids, vec![entities[0].id]);
    let first_id = entities[0].id;
    assert!(EditorSystem::select_only(&mut entities, Some(first_id)));

    let mut ui_system = UISystem::default();
    let events = ui_system.update_entities(&mut entities, Some((20.0, 20.0)), true, "PLAY");
    assert!(
        events
            .iter()
            .any(|event| event.entity_id == Some(entities[0].id))
    );
}

#[test]
fn animation_honors_state_speed_emits_events_and_stops_non_looping_clips() {
    let mut graphs = AnimationGraphLibrary::new();
    let controller = graphs.controllers.get_mut("Default").unwrap();
    let state = controller.states.get_mut("Idle").unwrap();
    state.speed = 2.0;
    state.looped = false;
    controller
        .clips
        .get_mut("Idle")
        .unwrap()
        .events
        .push(AnimationEvent {
            time: 0.1,
            name: "step".to_string(),
            payload: json!({"foot": "left"}),
        });
    let mut entity = GameObject::new(0.0, 0.0, Some("Animated".to_string()));
    entity.add_component(default_component("Animator").unwrap());
    let mut entities = vec![entity];

    let report = AnimationSystem.update_entities_with_report(&mut entities, &graphs, 0.1, "PLAY");
    let animator = entities[0].get_component("Animator").unwrap();
    assert_eq!(animator.get_f64("normalized_time", 0.0), 0.2);
    assert_eq!(report.events_emitted, 1);
    assert_eq!(
        animator.get("_events").unwrap().as_array().unwrap().len(),
        1
    );
}

#[test]
fn physics_and_render_use_spatial_broad_phases() {
    let mut entities: Vec<_> = (0..1_000)
        .map(|index| GameObject::new(index as f64 * 10.0, 0.0, Some(format!("Entity{index}"))))
        .collect();
    let mut physics = PhysicsSystem::new();
    physics.gravity = (0.0, 0.0);
    physics.update_entities_mut(&mut entities, 0.016, "PLAY");
    assert_eq!(physics.stats["broadphase_candidates"], 0);
    assert_eq!(physics.stats["broadphase_rejected"], 499_500);

    let mut camera = Camera::default();
    camera.set_viewport((0.0, 0.0, 30.0, 30.0));
    let mut render = RenderSystem::default();
    render.begin_frame();
    let stats = render.draw_camera(&entities, &camera);
    assert!(stats.visible_entities < entities.len());
    assert!(stats.culled_entities > 0);
}

#[test]
fn audio_command_history_is_bounded() {
    let mut audio = AudioSystem::default();
    for index in 0..300 {
        audio.play_sfx(&format!("sfx-{index}"), 1.0);
    }
    assert_eq!(audio.command_log.len(), 256);
    assert_eq!(audio.stats["dropped_audio_commands"], 44);

    let mut entity = GameObject::new(0.0, 0.0, Some("Audio".to_string()));
    let mut source = default_component("AudioSource").unwrap();
    source.set("audio_name", json!("loop"));
    source.set("play_on_start", json!(true));
    entity.add_component(source);
    audio.update_entities(&mut [entity], &AudioMixer::new(), "PLAY");
}

#[test]
fn camera_diagonal_pan_is_normalized_and_cursor_zoom_is_anchored() {
    let mut camera = Camera::default();
    camera.set_bounds(-10_000.0, -10_000.0, 10_000.0, 10_000.0);
    CameraSystem::pan(&mut camera, (1.0, 1.0), 0.1);
    assert!((camera.x.hypot(camera.y) - 52.0).abs() < 0.001);

    let cursor = (100.0, 80.0);
    let before = (
        camera.x + cursor.0 / camera.zoom,
        camera.y + cursor.1 / camera.zoom,
    );
    CameraSystem::zoom_towards(&mut camera, cursor, 1.0, 0.1);
    let after = (
        camera.x + cursor.0 / camera.zoom,
        camera.y + cursor.1 / camera.zoom,
    );
    assert!((before.0 - after.0).abs() < 0.001);
    assert!((before.1 - after.1).abs() < 0.001);
}
