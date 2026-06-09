---
id: 035
title: "reqwest Async HTTP Client"
tags: [rust, http, async, reqwest, network]
phase: 5
created: 2026-06-09
---

## What

reqwest is an ergonomic HTTP client for Rust. It supports async/await via tokio, connection pooling, and various body extraction methods (.text(), .bytes(), .json()).

## Why

BNVR needs to fetch subscription YAML from arbitrary URLs. The client must be async (runs inside tokio runtime), handle errors gracefully, and set a user agent.

## How

Build a reusable client:

```rust
let client = reqwest::Client::builder()
    .user_agent("bnvr")
    .build()?;
```

Fetch and check status:

```rust
let resp = client.get(url).send().await?;
let status = resp.status();
if !status.is_success() {
    return Err(format!("HTTP {status} from {url}").into());
}
let text = resp.text().await?;
```

**Key differences**:
- `.text()` - returns String, fails on non-UTF-8
- `.bytes()` - returns Bytes, works with any encoding
- `.json::<T>()` - deserializes JSON response body

**Gotcha**: The client should be reused across requests (connection pooling). Creating a new client per request wastes TLS handshakes.

## Links

- [019-github-api-reqwest](./019-github-api-reqwest.md) - earlier reqwest usage for GitHub API
- [033-serde-yaml-parsing](./033-serde-yaml-parsing.md) - parsing the fetched YAML
