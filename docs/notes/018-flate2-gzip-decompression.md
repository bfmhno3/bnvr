---
id: 018
title: "flate2 Gzip Decompression"
tags: [rust, compression, flate2]
phase: 4
created: 2026-06-08
---

## What

`flate2` is a Rust crate providing DEFLATE-based compression and decompression. We use it to extract `.gz` files downloaded from Mihomo GitHub Releases (Linux builds are gzipped single binaries).

## Why

Mihomo releases Linux binaries as `.gz` files (e.g. `mihomo-linux-amd64-v1.19.27.gz`). We need to decompress them in memory before writing the binary to disk.

## How

```rust
use flate2::read::GzDecoder;
use std::io::Read;

fn extract_gz(data: &[u8], dest: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut decoder = GzDecoder::new(data);
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf)?;
    let mut file = std::fs::File::create(dest)?;
    std::io::Write::write_all(&mut file, &buf)?;
    Ok(())
}
```

Key points:
- `GzDecoder` wraps any `Read` implementor and implements `Read` itself
- Call `read_to_end()` to decompress everything into a buffer
- The `.gz` contains a single file (the binary), not a tar archive

## Links

- [022-zip-extraction](./022-zip-extraction.md)
- [019-github-api-reqwest](./019-github-api-reqwest.md)
