---
id: 022
title: "Zip Archive Extraction"
tags: [rust, compression, zip]
phase: 4
created: 2026-06-08
---

## What

Using the `zip` crate to extract files from a zip archive in memory.

## Why

Mihomo releases Windows binaries as `.zip` files. We need to find and extract the `.exe` binary from the archive.

## How

```rust
use std::io;
use zip::ZipArchive;

fn extract_zip(data: &[u8], dest_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let reader = std::io::Cursor::new(data);
    let mut archive = ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name().ends_with(".exe") {
            let dest = dest_dir.join("mihomo.exe");
            let mut out = std::fs::File::create(&dest)?;
            io::copy(&mut file, &mut out)?;
            return Ok(());
        }
    }
    Err("no .exe found in archive".into())
}
```

Key points:
- `Cursor::new(data)` wraps `&[u8]` as a `Read + Seek` (required by `ZipArchive`)
- Iterate entries with `by_index()`, check `name()` to find the target
- `zip 2.x` is sufficient; `zip 8.x` is available but pulls in more deps
- Scan for the binary by pattern rather than hardcoding exact filenames

## Links

- [018-flate2-gzip-decompression](./018-flate2-gzip-decompression.md) -- Linux .gz equivalent
