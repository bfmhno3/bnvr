use super::crud;

pub fn view(name: Option<&str>, yaml_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let name = crud::resolve(name)?;
    let raw = crud::effective_config(&name)?;
    let yaml_val: serde_yaml::Value = serde_yaml::from_str(&raw)?;

    let target = match yaml_path {
        Some(path) => {
            navigate_path(&yaml_val, path).ok_or_else(|| format!("path not found: {path}"))?
        }
        None => &yaml_val,
    };

    print!("{}", serde_yaml::to_string(target)?);
    Ok(())
}

pub fn navigate_path<'a>(
    value: &'a serde_yaml::Value,
    path: &str,
) -> Option<&'a serde_yaml::Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = match current {
            serde_yaml::Value::Mapping(map) => map.get(segment)?,
            serde_yaml::Value::Sequence(seq) => seq.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_yaml() -> serde_yaml::Value {
        serde_yaml::from_str(
            "proxies:\n  - name: node1\n    type: ss\n  - name: node2\n    type: vmess\nrules:\n  - DOMAIN-SUFFIX,google.com,Proxy\nport: 7890\n",
        )
        .unwrap()
    }

    #[test]
    fn test_navigate_object_key() {
        let val = sample_yaml();
        let result = navigate_path(&val, "port").unwrap();
        assert_eq!(result, &serde_yaml::Value::Number(7890.into()));
    }

    #[test]
    fn test_navigate_array_index() {
        let val = sample_yaml();
        let result = navigate_path(&val, "proxies.0").unwrap();
        assert_eq!(
            navigate_path(result, "name"),
            Some(&serde_yaml::Value::String("node1".into()))
        );
    }

    #[test]
    fn test_navigate_nested() {
        let val = sample_yaml();
        let result = navigate_path(&val, "proxies.1.type").unwrap();
        assert_eq!(result, &serde_yaml::Value::String("vmess".into()));
    }

    #[test]
    fn test_navigate_empty_path() {
        let val = sample_yaml();
        let result = navigate_path(&val, "");
        assert!(result.is_none());
    }

    #[test]
    fn test_navigate_invalid_key() {
        let val = sample_yaml();
        assert!(navigate_path(&val, "nonexistent").is_none());
    }

    #[test]
    fn test_navigate_array_out_of_bounds() {
        let val = sample_yaml();
        assert!(navigate_path(&val, "proxies.99").is_none());
    }

    #[test]
    fn test_navigate_array_non_numeric() {
        let val = sample_yaml();
        assert!(navigate_path(&val, "proxies.abc").is_none());
    }
}
