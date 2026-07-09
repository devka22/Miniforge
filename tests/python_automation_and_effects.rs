use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use image::{ImageBuffer, Rgba};
use miniforge::engine::component::default_component;
use miniforge::engine::editor_python::{
    PythonAutomationHost, PythonEditorContext, batch_convert_sprites, generate_paged_sprite_atlases,
};
use miniforge::engine::render_2d::{PostProcessStack2D, production_effect_presets_2d};

fn temp_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("miniforge-python-suite-{label}-{nonce}"));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn production_python_suite_installs_and_queues_editor_operations() {
    let root = temp_root("tools");
    let host = PythonAutomationHost::new(&root);
    if host.interpreter_version().is_err() {
        return;
    }
    host.install_builtin_tools().unwrap();
    let tools = host.discover().unwrap();
    assert!(tools.len() >= 10);
    let atlas = tools
        .iter()
        .find(|tool| tool.id == "atlas_generator")
        .unwrap();
    let result = host.run(atlas, PythonEditorContext::default()).unwrap();
    assert!(result.success, "{}", result.stderr);
    assert_eq!(result.operations[0].operation, "generate_atlas");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn sprite_conversion_and_paged_atlas_are_real_file_operations() {
    let root = temp_root("atlas");
    let sprites = root.join("assets/sprites");
    fs::create_dir_all(&sprites).unwrap();
    for index in 0..12u8 {
        let image = ImageBuffer::from_pixel(18, 18, Rgba([index * 20, 80, 220, 255]));
        image
            .save(sprites.join(format!("sprite_{index}.bmp")))
            .unwrap();
    }
    let converted =
        batch_convert_sprites(&root, "assets/sprites", "assets/sprites_converted").unwrap();
    assert_eq!(converted.processed, 12);
    let atlas =
        generate_paged_sprite_atlases(&root, "assets/sprites_converted", "assets/atlases", 64, 1)
            .unwrap();
    assert_eq!(atlas.processed, 12);
    assert!(atlas.output_files.iter().any(|path| path.ends_with(".png")));
    assert!(
        atlas.output_files.len() >= 4,
        "expected multiple atlas pages"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn complete_2d_effect_suite_has_components_presets_and_post_process_defaults() {
    let components = [
        "Light2D",
        "ShadowCaster2D",
        "NormalMap2D",
        "Water2D",
        "Distortion2D",
        "Fire2D",
        "Fog2D",
        "Outline2D",
        "Bloom2D",
        "GpuParticles2D",
        "DamageEffect2D",
        "PixelArtShader2D",
    ];
    assert!(
        components
            .iter()
            .all(|name| default_component(name).is_some())
    );

    let presets = production_effect_presets_2d();
    assert_eq!(presets.len(), components.len());
    assert!(presets.iter().any(|preset| preset.id == "gpu_particles"));
    assert!(presets.iter().any(|preset| preset.id == "pixel_art"));

    let post = PostProcessStack2D::default();
    assert!(post.effects.iter().any(|effect| effect.name == "Bloom"));
    assert!(post.effects.iter().any(|effect| effect.name == "Outline"));
    assert!(post.effects.iter().any(|effect| effect.name == "Fog"));
}
