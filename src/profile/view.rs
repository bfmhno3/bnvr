use rusqlite::Connection;

use super::crud;

pub fn view(
    conn: &Connection,
    name: &str,
    json_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let profile = crud::get(conn, name)?;

    let raw = profile
        .raw_config
        .as_deref()
        .ok_or("no config stored for this profile (run `bnvr profile sync` first)")?;

    let yaml_val: serde_yaml::Value = serde_yaml::from_str(raw)?;
    let json_val: serde_json::Value = serde_json::to_value(yaml_val)?;

    let target = match json_path {
        Some(path) => {
            navigate_path(&json_val, path).ok_or_else(|| format!("path not found: {path}"))?
        }
        None => &json_val,
    };

    println!("{}", serde_json::to_string_pretty(target)?);
    Ok(())
}

pub fn navigate_path<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = match current {
            serde_json::Value::Object(map) => map.get(segment)?,
            serde_json::Value::Array(arr) => {
                let index: usize = segment.parse().ok()?;
                arr.get(index)?
            }
            _ => return None,
        };
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_json() -> serde_json::Value {
        serde_json::json!({
            "proxies": [
                {"name": "node1", "type": "ss"},
                {"name": "node2", "type": "vmess"}
            ],
            "rules": ["DOMAIN-SUFFIX,google.com,Proxy"],
            "port": 7890
        })
    }

    #[test]
    fn test_navigate_object_key() {
        let val = sample_json();
        let result = navigate_path(&val, "port").unwrap();
        assert_eq!(result, &serde_json::json!(7890));
    }

    #[test]
    fn test_navigate_array_index() {
        let val = sample_json();
        let result = navigate_path(&val, "proxies.0").unwrap();
        assert_eq!(result["name"], "node1");
    }

    #[test]
    fn test_navigate_nested() {
        let val = sample_json();
        let result = navigate_path(&val, "proxies.1.type").unwrap();
        assert_eq!(result, &serde_json::json!("vmess"));
    }

    #[test]
    fn test_navigate_empty_path() {
        let val = sample_json();
        // Empty path returns root
        let result = navigate_path(&val, "");
        // Empty split yields one empty segment, which won't match
        assert!(result.is_none());
    }

    #[test]
    fn test_navigate_invalid_key() {
        let val = sample_json();
        assert!(navigate_path(&val, "nonexistent").is_none());
    }

    #[test]
    fn test_navigate_array_out_of_bounds() {
        let val = sample_json();
        assert!(navigate_path(&val, "proxies.99").is_none());
    }

    #[test]
    fn test_navigate_array_non_numeric() {
        let val = sample_json();
        assert!(navigate_path(&val, "proxies.abc").is_none());
    }
}
