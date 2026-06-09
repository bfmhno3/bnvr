---
id: 036
title: "JSON Pointer Navigation (Dot-Path Access)"
tags: [rust, json, serde_json, data-structures]
phase: 5
created: 2026-06-09
---

## What

Navigating a `serde_json::Value` tree by splitting a dot-separated path string into segments and traversing objects by key and arrays by index.

## Why

`bnvr profile view proxies.0.name` lets users inspect specific parts of a large YAML config without opening an editor. The config is parsed into a serde_json::Value tree, then navigated by path.

## How

```rust
pub fn navigate_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
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
```

**Path examples**:
- `"proxies"` - object key access
- `"proxies.0"` - array index access
- `"proxies.0.name"` - nested traversal

**Gotcha**: An empty path string splits into one empty segment, which won't match any key. Handle this case explicitly if you want empty-path to mean "return root".

## Links

- [033-serde-yaml-parsing](./033-serde-yaml-parsing.md) - YAML to JSON conversion
