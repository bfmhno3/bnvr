---
id: 019
title: "GitHub API with reqwest"
tags: [rust, http, reqwest, github-api, async]
phase: 4
created: 2026-06-08
---

## What

Using async `reqwest` to call the GitHub REST API and deserialize JSON responses with `serde`.

## Why

BNVR needs to fetch the latest Mihomo release version from GitHub (`/repos/MetaCubeX/mihomo/releases/latest`) before downloading. The CLI runs inside `#[tokio::main]`, so we use async reqwest (blocking reqwest panics inside a tokio runtime).

## How

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

fn api_client() -> Result<reqwest::Client, Box<dyn std::error::Error>> {
    let mut builder = reqwest::Client::builder().user_agent("bnvr");
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        builder = builder.default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))?,
            );
            h
        });
    }
    Ok(builder.build()?)
}

async fn latest_version() -> Result<String, Box<dyn std::error::Error>> {
    let resp = api_client()?
        .get("https://api.github.com/repos/MetaCubeX/mihomo/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("GitHub API returned {status}: {body}").into());
    }

    let release: Release = resp.json().await?;
    Ok(release.tag_name)
}
```

Key points:
- Cannot use `reqwest::blocking` inside `#[tokio::main]` -- panics at runtime
- `reqwest` features needed: `["json", "rustls"]` (not `blocking`)
- `GITHUB_TOKEN` env var provides auth (avoids 60 req/hr rate limit)
- `Accept: application/vnd.github+json` header is recommended by GitHub
- Consume `resp.status()` before `resp.text()` -- response is moved on read

## Links

- [018-flate2-gzip-decompression](./018-flate2-gzip-decompression.md)
- [023-sync-vs-async-fs](./023-sync-vs-async-fs.md)
