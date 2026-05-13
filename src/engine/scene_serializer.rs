use serde_json::{Value, json};

use crate::engine::version::ENGINE_VERSION;

pub struct SceneSerializer;

impl SceneSerializer {
    pub fn migrate(mut data: Value) -> Value {
        if let Some(objects) = data.get("objects").cloned()
            && let Some(map) = data.as_object_mut()
        {
            map.remove("objects");
            map.insert("entities".to_string(), objects);
        }
        let Some(map) = data.as_object_mut() else {
            return json!({});
        };
        map.entry("version".to_string())
            .or_insert(json!(ENGINE_VERSION));
        map.entry("engine_version".to_string())
            .or_insert(json!(ENGINE_VERSION));
        map.entry("scene_name".to_string()).or_insert(json!("main"));
        map.entry("mode".to_string()).or_insert(json!("EDITOR"));
        map.entry("active_tool".to_string())
            .or_insert(json!("Select"));
        map.entry("tile_brush".to_string()).or_insert(json!(0));
        map.entry("brush_size".to_string()).or_insert(json!(1));
        map.entry("camera".to_string())
            .or_insert(json!({"x": 0, "y": 0, "zoom": 1.0}));
        map.entry("control_groups".to_string()).or_insert(json!({}));
        map.entry("grid".to_string()).or_insert(Value::Null);
        map.entry("tiles".to_string()).or_insert(json!([]));
        map.entry("settings".to_string()).or_insert(json!({}));
        map.entry("entities".to_string()).or_insert(json!([]));
        map.entry("editor_view_settings".to_string())
            .or_insert(json!({}));
        map.entry("ui_canvases".to_string()).or_insert(json!([]));
        data
    }
}
