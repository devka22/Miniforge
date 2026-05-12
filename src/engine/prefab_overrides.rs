use serde_json::{Value, json};

#[derive(Debug, Clone, Default)]
pub struct PrefabOverrides;

impl PrefabOverrides {
    pub fn diff_dict(path: &str, before: &Value, after: &Value) -> Vec<Value> {
        let mut diffs = Vec::new();
        match (before, after) {
            (Value::Object(a), Value::Object(b)) => {
                for key in a.keys().chain(b.keys()) {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    match (a.get(key), b.get(key)) {
                        (Some(left), Some(right)) => {
                            diffs.extend(Self::diff_dict(&child_path, left, right));
                        }
                        (left, right) => diffs.push(json!({
                            "path": child_path,
                            "before": left.cloned().unwrap_or(Value::Null),
                            "after": right.cloned().unwrap_or(Value::Null),
                        })),
                    }
                }
            }
            _ if before != after => diffs.push(json!({
                "path": path,
                "before": before,
                "after": after,
            })),
            _ => {}
        }
        diffs
    }
}
