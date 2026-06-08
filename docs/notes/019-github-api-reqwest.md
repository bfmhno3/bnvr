---
id: 019
title: "GitHub API with reqwest"
tags: [rust, http, reqwest, github-api]
phase: 4
created: 2026-06-08
---

## What

Using `reqwest::blocking` to call the GitHub REST API and deserialize JSON responses with `serde`.

## Why

BNVR needs to fetch the latest Mihomo release version from GitHub (`/repos/MetaCubeX/mihomo/releases/latest`) before downloading.

## How

```rust
use reqwest::blocking::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

fn latest_version() -> Result<String, Box<dyn std::error::Error>> {
    let resp = Client::builder()
        .user_agent("bnvr")
        .build()?
        .get("https://api.github.com/repos/MetaCubeX/mihomo/releases/latest")
        .send()?;

    let release: Release = resp.json()?;
    Ok(release.tag_name)
}
```

Key points:
- GitHub API requires a `User-Agent` header (403 without it)
- `reqwest` needs `features = ["blocking", "json"]` in Cargo.toml
- `resp.json::<T>()` auto-deserializes when `T: DeserializeOwned`
- Rate limit: 60 requests/hour unauthenticated

## Links

- [018-flate2-gzip-decompression](./018-flate2-gzip-decompression.md)
- [001-cargo-toml-dependencies](./001-cargo-toml-dependencies.md)
