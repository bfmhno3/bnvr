---
id: 010
title: "JSON Message Protocol"
tags: [rust, serde, ipc, protocol]
phase: 2
created: 2026-06-08
---

## What

A simple request/response protocol over byte streams. Each message is a JSON object followed by `\n` (newline-delimited JSON). Requests have `id`, `method`, `params`. Responses have `id`, `result`, `error`.

## Why

The daemon and client communicate over a local socket. JSON is human-readable and easy to debug. The `id` field correlates responses to requests.

## How

```rust
#[derive(Serialize, Deserialize)]
struct Request {
    id: u64,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct Response {
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<String>,
}
```

Read with `BufReader::read_line`, write with `serde_json::to_string` + `\n`.

`#[serde(default)]` on `params` makes it optional -- requests without params omit the field.

`#[serde(skip_serializing_if = "Option::is_none")]` on `result`/`error` omits null fields from the output.

## Gotchas

- `read_line` includes the `\n` in the returned string. `serde_json::from_str` handles trailing whitespace.
- No message size limit yet. A malicious client could send an unbounded line. Consider `take(N)` on the reader.
- The protocol is synchronous: one request, one response. Multiplexing would need request IDs and concurrent dispatch.

## Links

- [011-interprocess-ipc](./011-interprocess-ipc.md)
