---
id: 033
title: "serde_yaml for Rust YAML Parsing"
tags: [rust, yaml, serde, serialization]
phase: 5
created: 2026-06-09
---

## What

serde_yaml is a Rust crate that provides YAML serialization and deserialization using serde. It can parse YAML text into Rust types or into an untyped `serde_yaml::Value` tree, and serialize Rust types back to YAML.

## Why

Mihomo proxy configs are distributed as YAML. BNVR needs to fetch, parse, and inspect these configs. The Python bridge (Phase 6) works with JSON, so we also need to convert between YAML and JSON representations.

## How

Parse YAML into an untyped value tree:

```rust
let yaml_val: serde_yaml::Value = serde_yaml::from_str(yaml_text)?;
```

Convert to serde_json::Value for JSON path navigation:

```rust
let json_val: serde_json::Value = serde_json::to_value(yaml_val)?;
```

Parse directly into a typed struct:

```rust
#[derive(Deserialize)]
struct Config {
    proxies: Vec<Proxy>,
    rules: Vec<String>,
}

let config: Config = serde_yaml::from_str(yaml_text)?;
```

**Gotcha**: serde_yaml 0.9 is marked deprecated on crates.io but remains stable and widely used. The maintained alternative is `serde_yml`, but serde_yaml works fine for our needs.

## Links

- [019-github-api-reqwest](./019-github-api-reqwest.md) - reqwest for HTTP fetching
- [036-json-pointer-navigation](./036-json-pointer-navigation.md) - navigating the parsed value tree
