use std::collections::BTreeSet;

use serde_json::{Value, json};

#[derive(Debug, Clone, Default)]
pub struct PrefabOverrides;

impl PrefabOverrides {
    pub fn diff_dict(path: &str, before: &Value, after: &Value) -> Vec<Value> {
        let mut diffs = Vec::new();
        match (before, after) {
            (Value::Object(a), Value::Object(b)) => {
                let keys = a.keys().chain(b.keys()).cloned().collect::<BTreeSet<_>>();
                for key in keys {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    match (a.get(&key), b.get(&key)) {
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

    pub fn apply(base: &mut Value, overrides: &[Value]) -> Result<usize, String> {
        let mut applied = 0;
        for change in overrides {
            let path = change
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| "prefab override is missing `path`".to_string())?;
            let value = change.get("after").cloned().unwrap_or(Value::Null);
            set_path(base, path, value)?;
            applied += 1;
        }
        Ok(applied)
    }
}

fn set_path(root: &mut Value, path: &str, value: Value) -> Result<(), String> {
    let segments = path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let Some((last, parents)) = segments.split_last() else {
        *root = value;
        return Ok(());
    };
    let mut current = root;
    for segment in parents {
        current = current
            .as_object_mut()
            .and_then(|object| object.get_mut(*segment))
            .ok_or_else(|| format!("prefab override path does not exist: {path}"))?;
    }
    let object = current
        .as_object_mut()
        .ok_or_else(|| format!("prefab override parent is not an object: {path}"))?;
    object.insert((*last).to_string(), value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_has_unique_paths_and_can_be_applied() {
        let before = json!({"transform":{"x":1,"y":2},"name":"Hero"});
        let after = json!({"transform":{"x":4,"y":2},"name":"Hero 2"});
        let diff = PrefabOverrides::diff_dict("", &before, &after);
        assert_eq!(diff.len(), 2);
        let paths = diff
            .iter()
            .filter_map(|change| change.get("path").and_then(Value::as_str))
            .collect::<BTreeSet<_>>();
        assert_eq!(paths.len(), 2);
        let mut applied = before;
        assert_eq!(PrefabOverrides::apply(&mut applied, &diff).unwrap(), 2);
        assert_eq!(applied, after);
    }
}
