use std::collections::BTreeMap;

use miniforge::engine::component::default_component;
use miniforge::entities::game_object::GameObject;
use miniforge::map::grid::Grid;
use miniforge::render::backend::{
    BUILTIN_RADIAL_LIGHT_TEXTURE_ID, ParticleDrawCommand, RenderBackend, RenderTargetDescriptor2D,
    SpriteBlendMode, SpriteDrawCommand, SpriteDrawOptions, SpriteMaterialEffect,
    SpriteRegionDrawCommand, TextDrawCommand, TextWrapMode, WgpuBackend,
};
use miniforge::render::runtime_scene_2d::{
    RenderTargetCameraView2D, draw_engine_runtime_scene_2d,
    draw_engine_runtime_world_to_render_target_2d,
};
use miniforge::runtime::engine_runtime::EngineRuntime;
use serde_json::json;

#[test]
#[ignore = "requires a physical or software wgpu adapter"]
fn physical_wgpu_backend_renders_and_reads_pixels() {
    let mut backend = WgpuBackend::new(true, cfg!(target_os = "macos"));
    backend.resize(64, 64).unwrap();
    backend.set_clear_color([0.0, 0.0, 0.0, 1.0]);
    backend.init().unwrap();
    backend
        .upload_texture_rgba8(7, 2, 1, &[0, 255, 0, 255, 0, 0, 255, 255])
        .unwrap();
    backend.begin_frame().unwrap();
    backend
        .draw_sprite(SpriteDrawCommand {
            entity_id: 1,
            texture_id: 0,
            x: 16.0,
            y: 16.0,
            width: 32.0,
            height: 32.0,
            rotation: 0.0,
            color: [1.0, 0.0, 0.0, 1.0],
        })
        .unwrap();
    backend
        .draw_sprite_region(SpriteRegionDrawCommand {
            sprite: SpriteDrawCommand {
                entity_id: 2,
                texture_id: 7,
                x: 48.0,
                y: 0.0,
                width: 16.0,
                height: 16.0,
                rotation: 0.0,
                color: [1.0, 1.0, 1.0, 1.0],
            },
            uv_rect: [0.5, 0.0, 1.0, 1.0],
            clip_rect: Some([52, 0, 12, 16]),
        })
        .unwrap();
    backend
        .draw_sprite_with_options(
            SpriteDrawCommand {
                entity_id: 3,
                texture_id: BUILTIN_RADIAL_LIGHT_TEXTURE_ID,
                x: 32.0,
                y: 0.0,
                width: 16.0,
                height: 16.0,
                rotation: 0.0,
                color: [1.0, 0.8, 0.45, 1.0],
            },
            SpriteDrawOptions {
                blend_mode: SpriteBlendMode::Additive,
                ..SpriteDrawOptions::default()
            },
        )
        .unwrap();
    for (index, blend_mode) in [
        SpriteBlendMode::Additive,
        SpriteBlendMode::Multiply,
        SpriteBlendMode::Screen,
        SpriteBlendMode::PremultipliedAlpha,
    ]
    .into_iter()
    .enumerate()
    {
        let x = index as f32 * 16.0;
        backend
            .draw_sprite(SpriteDrawCommand {
                entity_id: 10 + index as u64 * 2,
                texture_id: 0,
                x,
                y: 48.0,
                width: 16.0,
                height: 16.0,
                rotation: 0.0,
                color: [0.35, 0.35, 0.35, 1.0],
            })
            .unwrap();
        backend
            .draw_sprite_with_options(
                SpriteDrawCommand {
                    entity_id: 11 + index as u64 * 2,
                    texture_id: 0,
                    x,
                    y: 48.0,
                    width: 16.0,
                    height: 16.0,
                    rotation: 0.0,
                    color: [0.25, 0.75, 0.5, 0.65],
                },
                SpriteDrawOptions {
                    blend_mode,
                    ..SpriteDrawOptions::default()
                },
            )
            .unwrap();
    }
    backend
        .draw_sprite_with_options(
            SpriteDrawCommand {
                entity_id: 30,
                texture_id: 0,
                x: 48.0,
                y: 16.0,
                width: 16.0,
                height: 16.0,
                rotation: 0.0,
                color: [1.0, 0.0, 0.0, 1.0],
            },
            SpriteDrawOptions {
                material_effect: SpriteMaterialEffect::Grayscale,
                ..SpriteDrawOptions::default()
            },
        )
        .unwrap();
    backend
        .draw_text(TextDrawCommand {
            text_id: 99,
            text: "GPU".to_string(),
            font_family: String::new(),
            x: 0.0,
            y: 32.0,
            width: 16.0,
            height: 16.0,
            font_size: 12.0,
            line_height: 14.0,
            color: [255, 255, 255, 255],
            wrap: TextWrapMode::None,
            clip_rect: Some([0, 32, 16, 16]),
        })
        .unwrap();
    backend
        .draw_particles(ParticleDrawCommand {
            system_id: 77,
            particle_count: 64,
            origin: [8.0, 8.0],
            velocity: [0.0, 0.0],
            gravity: [0.0, 0.0],
            spread: 0.0,
            lifetime: 2.0,
            start_size: 10.0,
            end_size: 10.0,
            color: [0.0, 1.0, 0.0, 1.0],
            emission_rate: 0.0,
            burst_count: 16,
            blend_mode: SpriteBlendMode::Alpha,
            ..ParticleDrawCommand::default()
        })
        .unwrap();
    backend.end_frame().unwrap();

    let pixels = backend.readback_rgba8().unwrap();
    let center = (32 * 64 + 32) * 4;
    let corner = (2 * 64 + 2) * 4;
    let clipped_out = (8 * 64 + 50) * 4;
    let textured = (8 * 64 + 56) * 4;
    let light_center = (8 * 64 + 40) * 4;
    let light_edge = 32 * 4;
    let grayscale = (24 * 64 + 56) * 4;
    let gpu_particle = (8 * 64 + 8) * 4;
    assert!(pixels[center] > 240, "center should be rendered red");
    assert!(pixels[center + 1] < 16);
    assert!(pixels[center + 2] < 16);
    assert!(pixels[corner] < 16, "corner should preserve clear color");
    assert!(
        pixels[clipped_out + 2] < 16,
        "atlas sprite should respect its clip rectangle"
    );
    assert!(
        pixels[textured + 2] > 240,
        "uploaded atlas region should sample the blue texel"
    );
    assert!(
        pixels[light_center] > pixels[light_edge] + 64,
        "built-in light texture should produce a soft radial falloff"
    );
    assert!(
        pixels[grayscale].abs_diff(pixels[grayscale + 1]) < 3
            && pixels[grayscale + 1].abs_diff(pixels[grayscale + 2]) < 3,
        "grayscale material should transform the sprite in WGSL"
    );
    assert!(
        pixels[gpu_particle + 1] > 200,
        "compute particles should simulate and render through the physical GPU"
    );
    for x in [8usize, 24, 40, 56] {
        let blended = (56 * 64 + x) * 4;
        assert!(
            pixels[blended..blended + 3]
                .iter()
                .any(|channel| *channel > 32),
            "every blend pipeline should write visible color"
        );
    }
    assert!(
        (32usize..48).any(|y| {
            (0usize..16).any(|x| {
                let pixel = (y * 64 + x) * 4;
                pixels[pixel..pixel + 3].iter().any(|channel| *channel > 32)
            })
        }),
        "glyph atlas should render clipped UI text"
    );
    assert_eq!(backend.submitted_frames, 1);
    assert!(backend.is_using_physical_device());
    assert_eq!(backend.texture_count(), 1);
    assert_eq!(backend.last_frame_diagnostics().queued_text_areas, 1);
    assert_eq!(backend.last_frame_diagnostics().queued_particle_systems, 1);
    assert_eq!(backend.last_frame_diagnostics().gpu_particle_capacity, 64);
    assert_eq!(backend.last_frame_diagnostics().gpu_particle_spawned, 16);
    assert_eq!(
        backend.last_frame_diagnostics().particle_compute_dispatches,
        1
    );
    assert!(
        backend.last_frame_diagnostics().pipeline_changes >= 8,
        "alpha/effect alternation should switch pipelines without reordering"
    );

    backend.begin_frame().unwrap();
    backend
        .draw_particles(ParticleDrawCommand {
            system_id: 77,
            particle_count: 64,
            origin: [8.0, 8.0],
            velocity: [0.0, 0.0],
            gravity: [0.0, 0.0],
            spread: 0.0,
            lifetime: 2.0,
            start_size: 10.0,
            end_size: 10.0,
            color: [0.0, 1.0, 0.0, 1.0],
            emission_rate: 0.0,
            burst_count: 16,
            blend_mode: SpriteBlendMode::Alpha,
            ..ParticleDrawCommand::default()
        })
        .unwrap();
    backend.end_frame().unwrap();
    let persisted = backend.readback_rgba8().unwrap();
    assert!(
        persisted[gpu_particle + 1] > 200,
        "particle storage must remain resident across frames"
    );
    assert_eq!(backend.last_frame_diagnostics().gpu_particle_spawned, 0);
    assert_eq!(backend.submitted_frames, 2);
}

#[test]
#[ignore = "requires a physical or software wgpu adapter"]
fn physical_wgpu_normal_maps_react_to_tangent_space_light_direction() {
    let mut backend = WgpuBackend::new(true, cfg!(target_os = "macos"));
    backend.resize(64, 32).unwrap();
    backend.set_clear_color([0.0, 0.0, 0.0, 1.0]);
    backend.init().unwrap();
    backend
        .upload_texture_rgba8(41, 1, 1, &[255, 128, 128, 255])
        .unwrap();

    backend.begin_frame().unwrap();
    for (entity_id, x, light_x) in [(1, 0.0, i16::MAX), (2, 32.0, -i16::MAX)] {
        backend
            .draw_sprite_with_options(
                SpriteDrawCommand {
                    entity_id,
                    texture_id: 0,
                    x,
                    y: 0.0,
                    width: 32.0,
                    height: 32.0,
                    rotation: 0.0,
                    color: [1.0; 4],
                },
                SpriteDrawOptions {
                    normal_texture_id: Some(41),
                    normal_strength: u8::MAX,
                    light_direction: [light_x, 0],
                    light_color: [u8::MAX; 3],
                    ambient_light: 16,
                    ..SpriteDrawOptions::default()
                },
            )
            .unwrap();
    }
    backend.end_frame().unwrap();

    let pixels = backend.readback_rgba8().unwrap();
    let luma = |x: usize| {
        let offset = (16 * 64 + x) * 4;
        u16::from(pixels[offset]) + u16::from(pixels[offset + 1]) + u16::from(pixels[offset + 2])
    };
    let facing = luma(16);
    let opposing = luma(48);
    assert!(
        facing > opposing + 300,
        "right-facing normal should receive substantially more light: facing={facing}, opposing={opposing}"
    );
    assert_eq!(
        backend.last_frame_diagnostics().normal_texture_bind_changes,
        1
    );
    assert!(backend.is_using_physical_device());
}

#[test]
#[ignore = "requires a physical or software wgpu adapter"]
fn physical_wgpu_render_target_draws_offscreen_then_samples_on_main_target() {
    let mut backend = WgpuBackend::new(true, cfg!(target_os = "macos"));
    backend.resize(64, 32).unwrap();
    backend.set_clear_color([0.0, 0.0, 0.0, 1.0]);
    backend.init().unwrap();
    backend
        .create_render_target_2d(RenderTargetDescriptor2D {
            texture_id: 51,
            width: 16,
            height: 16,
            clear_color: [0.0, 0.0, 0.0, 1.0],
            label: "Physical camera texture".to_string(),
        })
        .unwrap();

    backend.begin_frame().unwrap();
    backend.begin_render_target_2d(51).unwrap();
    backend
        .draw_sprite(SpriteDrawCommand {
            entity_id: 1,
            texture_id: 0,
            x: 0.0,
            y: 0.0,
            width: 16.0,
            height: 16.0,
            rotation: 0.0,
            color: [0.1, 0.8, 0.25, 1.0],
        })
        .unwrap();
    assert!(
        backend
            .draw_sprite(SpriteDrawCommand {
                entity_id: 2,
                texture_id: 51,
                x: 0.0,
                y: 0.0,
                width: 16.0,
                height: 16.0,
                rotation: 0.0,
                color: [1.0; 4],
            })
            .is_err(),
        "a target must reject sampling its own attachment in the same pass"
    );
    backend.end_render_target_2d().unwrap();
    backend
        .draw_sprite(SpriteDrawCommand {
            entity_id: 3,
            texture_id: 51,
            x: 16.0,
            y: 0.0,
            width: 32.0,
            height: 32.0,
            rotation: 0.0,
            color: [1.0; 4],
        })
        .unwrap();
    backend.end_frame().unwrap();

    let target_pixels = backend.readback_render_target_rgba8(51).unwrap();
    let target_center = (8 * 16 + 8) * 4;
    assert!(target_pixels[target_center + 1] > 200);
    let main_pixels = backend.readback_rgba8().unwrap();
    let main_center = (16 * 64 + 32) * 4;
    assert!(main_pixels[main_center + 1] > 200);
    assert!(main_pixels[main_center] < 100);
    assert_eq!(backend.render_target_count(), 1);
    assert_eq!(backend.last_frame_diagnostics().render_target_passes, 1);
    assert_eq!(backend.last_frame_diagnostics().gpu_draw_calls, 2);

    backend.begin_frame().unwrap();
    backend
        .draw_sprite(SpriteDrawCommand {
            entity_id: 4,
            texture_id: 0,
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 8.0,
            rotation: 0.0,
            color: [1.0; 4],
        })
        .unwrap();
    assert!(
        backend.begin_render_target_2d(51).is_err(),
        "off-screen passes must reject ambiguous submission after main-frame geometry"
    );
    backend.end_frame().unwrap();
}

#[test]
#[ignore = "requires a physical or software wgpu adapter"]
fn physical_wgpu_runtime_world_camera_renders_to_sampleable_target_without_ui() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("miniforge-wgpu-camera-target-{unique}"));
    std::fs::create_dir_all(root.join("assets/scenes")).unwrap();
    std::fs::create_dir_all(root.join("settings")).unwrap();
    std::fs::write(
        root.join("project.mforge"),
        r#"{"name":"Camera Target Test","start_scene":"assets/scenes/main.scene.json"}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("assets/scenes/main.scene.json"),
        r#"{"entities":[],"grid":{"width":4,"height":4,"tile_size":8,"chunk_size":2}}"#,
    )
    .unwrap();
    let mut runtime = EngineRuntime::new(&root).unwrap();
    runtime.grid = Grid::new(4, 4, 8, 2);
    let mut subject = GameObject::new(2.0, 2.0, Some("Camera Subject".to_string()));
    subject.tag = "Player".to_string();
    subject
        .get_component_mut("SpriteRenderer")
        .unwrap()
        .set("tint", json!([20, 80, 255, 255]));
    runtime.runtime_world.replace_entities(vec![subject]);

    let mut backend = WgpuBackend::new(true, cfg!(target_os = "macos"));
    backend.resize(64, 32).unwrap();
    backend.set_clear_color([0.0, 0.0, 0.0, 1.0]);
    backend.init().unwrap();
    backend
        .create_render_target_2d(RenderTargetDescriptor2D {
            texture_id: 61,
            width: 32,
            height: 32,
            clear_color: [0.0, 0.0, 0.0, 1.0],
            label: "Runtime world camera".to_string(),
        })
        .unwrap();

    backend.begin_frame().unwrap();
    let stats = draw_engine_runtime_world_to_render_target_2d(
        &mut backend,
        &runtime,
        &BTreeMap::new(),
        61,
        RenderTargetCameraView2D {
            world_x: 2.0,
            world_y: 2.0,
            width: 32,
            height: 32,
            ..RenderTargetCameraView2D::default()
        },
    )
    .unwrap();
    backend
        .draw_sprite(SpriteDrawCommand {
            entity_id: 3,
            texture_id: 61,
            x: 16.0,
            y: 0.0,
            width: 32.0,
            height: 32.0,
            rotation: 0.0,
            color: [1.0; 4],
        })
        .unwrap();
    backend.end_frame().unwrap();

    let pixels = backend.readback_rgba8().unwrap();
    let center = (16 * 64 + 32) * 4;
    assert!(
        pixels[center + 2] > pixels[center],
        "the centered Player entity should remain blue after world-camera render and composition"
    );
    assert_eq!(stats.tile_quads, 16);
    assert_eq!(stats.entity_quads, 1);
    assert_eq!(stats.ui_quads, 0);
    assert_eq!(backend.last_frame_diagnostics().render_target_passes, 1);
    std::fs::remove_dir_all(root).ok();
}

#[test]
#[ignore = "requires a physical or software wgpu adapter"]
fn physical_wgpu_runtime_composes_ambient_directional_and_shadow_lighting() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("miniforge-wgpu-lighting-{unique}"));
    std::fs::create_dir_all(root.join("assets/scenes")).unwrap();
    std::fs::create_dir_all(root.join("settings")).unwrap();
    std::fs::write(
        root.join("project.mforge"),
        r#"{"name":"Lighting Test","start_scene":"assets/scenes/main.scene.json"}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("assets/scenes/main.scene.json"),
        r#"{"entities":[],"grid":{"width":4,"height":4,"tile_size":16,"chunk_size":2}}"#,
    )
    .unwrap();
    let mut runtime = EngineRuntime::new(&root).unwrap();
    runtime.grid = Grid::new(4, 4, 16, 2);

    let light_entity =
        |name: &str, light_type: &str, x: f64, y: f64, intensity: f64, visible: bool| {
            let mut entity = GameObject::new(x, y, Some(name.to_string()));
            entity.visible = visible;
            let mut light = default_component("Light2D").unwrap();
            light.set("light_type", json!(light_type));
            light.set("intensity", json!(intensity));
            light.set("radius", json!(4.0));
            light.set("color", json!([230, 235, 255]));
            entity.add_component(light);
            entity
        };
    let ambient = light_entity("Night", "ambient", 0.0, 0.0, 0.7, false);
    let directional = light_entity("Moon", "directional", 0.0, 0.0, 0.6, false);
    let point = light_entity("Lamp", "point", 2.0, 2.0, 1.0, false);
    let mut caster = GameObject::new(3.0, 2.0, Some("Wall".to_string()));
    caster.add_component(default_component("ShadowCaster2D").unwrap());
    runtime
        .runtime_world
        .replace_entities(vec![ambient, directional, point, caster]);

    let mut backend = WgpuBackend::new(true, cfg!(target_os = "macos"));
    backend.resize(64, 64).unwrap();
    backend.set_clear_color([0.0, 0.0, 0.0, 1.0]);
    backend.init().unwrap();
    backend.begin_frame().unwrap();
    let stats =
        draw_engine_runtime_scene_2d(&mut backend, &runtime, &BTreeMap::new(), 64.0, 64.0).unwrap();
    backend.end_frame().unwrap();

    let pixels = backend.readback_rgba8().unwrap();
    let luma = |x: usize, y: usize| {
        let pixel = (y * 64 + x) * 4;
        i32::from(pixels[pixel]) + i32::from(pixels[pixel + 1]) + i32::from(pixels[pixel + 2])
    };
    let max_pair_darkening = (24usize..40)
        .flat_map(|y| (4usize..31).map(move |x| (x, y)))
        .map(|(x, y)| luma(x, y) - luma(64 - x, y))
        .max()
        .unwrap_or_default();
    assert_eq!(stats.ambient_light_quads, 1);
    assert_eq!(stats.directional_light_quads, 1);
    assert_eq!(stats.shadow_quads, 1);
    assert!(
        max_pair_darkening > 12,
        "projected shadow should darken at least one symmetric framebuffer pair: max_darkening={max_pair_darkening}"
    );
    assert!(backend.is_using_physical_device());
    std::fs::remove_dir_all(root).ok();
}

#[test]
#[ignore = "requires a physical or software wgpu adapter"]
fn physical_wgpu_runtime_loads_and_clips_retained_ui_documents() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("miniforge-wgpu-retained-ui-{unique}"));
    std::fs::create_dir_all(root.join("assets/scenes")).unwrap();
    std::fs::create_dir_all(root.join("assets/ui")).unwrap();
    std::fs::create_dir_all(root.join("settings")).unwrap();
    std::fs::write(
        root.join("project.mforge"),
        r#"{"name":"Retained UI Test","start_scene":"assets/scenes/main.scene.json"}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("assets/scenes/main.scene.json"),
        r#"{"entities":[],"grid":{"width":4,"height":4,"tile_size":16,"chunk_size":2}}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("assets/ui/hud.ui2d.json"),
        r#"{
            "name":"HUD",
            "viewport_width":64.0,
            "viewport_height":64.0,
            "theme":{"name":"Test","styles":{}},
            "widgets":[{
                "id":"clip",
                "widget_type":"ScrollBox",
                "rect":{"x":8.0,"y":8.0,"width":24.0,"height":16.0},
                "anchors":{"min_x":0.0,"min_y":0.0,"max_x":0.0,"max_y":0.0},
                "properties":{"content_height":40.0,"show_scrollbar":false},
                "style":{"background":[0,0,0,0]},
                "children":[{
                    "id":"red_panel",
                    "widget_type":"Panel",
                    "rect":{"x":4.0,"y":8.0,"width":16.0,"height":20.0},
                    "anchors":{"min_x":0.0,"min_y":0.0,"max_x":0.0,"max_y":0.0},
                    "style":{"background":[255,0,0,255]}
                }]
            }]
        }"#,
    )
    .unwrap();
    let mut runtime = EngineRuntime::new(&root).unwrap();
    let mut canvas_entity = GameObject::new(0.0, 0.0, Some("HUD".to_string()));
    let mut canvas_component = default_component("WidgetCanvas2D").unwrap();
    canvas_component.set("canvas", json!("assets/ui/hud.ui2d.json"));
    canvas_entity.add_component(canvas_component);
    runtime.runtime_world.replace_entities(vec![canvas_entity]);
    let report = runtime.reload_ui_documents();
    assert_eq!(report.loaded, 1);
    assert!(report.errors.is_empty());

    let mut backend = WgpuBackend::new(true, cfg!(target_os = "macos"));
    backend.resize(64, 64).unwrap();
    backend.set_clear_color([0.0, 0.0, 0.0, 1.0]);
    backend.init().unwrap();
    backend.begin_frame().unwrap();
    let stats =
        draw_engine_runtime_scene_2d(&mut backend, &runtime, &BTreeMap::new(), 64.0, 64.0).unwrap();
    backend.end_frame().unwrap();

    let pixels = backend.readback_rgba8().unwrap();
    let inside = (20 * 64 + 16) * 4;
    let clipped_out = (28 * 64 + 16) * 4;
    assert_eq!(stats.retained_ui_widgets, 2);
    assert_eq!(stats.retained_ui_quads, 1);
    assert_eq!(stats.retained_ui_clipped_quads, 1);
    assert!(
        pixels[inside] > 240 && pixels[inside + 1] < 16,
        "retained panel should render inside the ScrollBox clip"
    );
    assert!(
        pixels[clipped_out] < 80,
        "retained panel must not leak below the ScrollBox clip"
    );
    assert!(backend.is_using_physical_device());
    std::fs::remove_dir_all(root).ok();
}
