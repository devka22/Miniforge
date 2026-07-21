use miniforge::render::backend::{RenderBackend, SpriteDrawCommand, WgpuBackend};

#[test]
#[ignore = "requires a physical or software wgpu adapter"]
fn physical_wgpu_backend_renders_and_reads_pixels() {
    let mut backend = WgpuBackend::new(true, cfg!(target_os = "macos"));
    backend.resize(64, 64).unwrap();
    backend.set_clear_color([0.0, 0.0, 0.0, 1.0]);
    backend.init().unwrap();
    backend
        .upload_texture_rgba8(7, 1, 1, &[0, 255, 0, 255])
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
        .draw_sprite(SpriteDrawCommand {
            entity_id: 2,
            texture_id: 7,
            x: 48.0,
            y: 0.0,
            width: 16.0,
            height: 16.0,
            rotation: 0.0,
            color: [1.0, 1.0, 1.0, 1.0],
        })
        .unwrap();
    backend.end_frame().unwrap();

    let pixels = backend.readback_rgba8().unwrap();
    let center = (32 * 64 + 32) * 4;
    let corner = (2 * 64 + 2) * 4;
    let textured = (8 * 64 + 56) * 4;
    assert!(pixels[center] > 240, "center should be rendered red");
    assert!(pixels[center + 1] < 16);
    assert!(pixels[center + 2] < 16);
    assert!(pixels[corner] < 16, "corner should preserve clear color");
    assert!(
        pixels[textured + 1] > 240,
        "uploaded texture should be sampled"
    );
    assert_eq!(backend.submitted_frames, 1);
    assert!(backend.is_using_physical_device());
    assert_eq!(backend.texture_count(), 1);
}
