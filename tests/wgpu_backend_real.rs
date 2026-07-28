use miniforge::render::backend::{
    RenderBackend, SpriteBlendMode, SpriteDrawCommand, SpriteDrawOptions, SpriteRegionDrawCommand,
    TextDrawCommand, TextWrapMode, WgpuBackend,
};

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
                SpriteDrawOptions { blend_mode },
            )
            .unwrap();
    }
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
    backend.end_frame().unwrap();

    let pixels = backend.readback_rgba8().unwrap();
    let center = (32 * 64 + 32) * 4;
    let corner = (2 * 64 + 2) * 4;
    let clipped_out = (8 * 64 + 50) * 4;
    let textured = (8 * 64 + 56) * 4;
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
    assert!(
        backend.last_frame_diagnostics().pipeline_changes >= 8,
        "alpha/effect alternation should switch pipelines without reordering"
    );
}
