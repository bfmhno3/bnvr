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
        let name = file.name().to_string();
        // Match: mihomo.exe (Windows) or mihomo (Linux, no path separator, no extension)
        if name.ends_with(".exe")
            || (!name.contains('/') && !name.contains('.') && name.contains("mihomo"))
        {
            let dest = if name.ends_with(".exe") {
                dest_dir.join("mihomo.exe")
            } else {
                dest_dir.join("mihomo")
            };
            let mut out = std::fs::File::create(&dest)?;
            io::copy(&mut file, &mut out)?;
            return Ok(());
        }
    }
    Err("no mihomo binary found in zip archive".into())
}
```

Key points:
- `Cursor::new(data)` wraps `&[u8]` as a `Read + Seek` (required by `ZipArchive`)
- Iterate entries with `by_index()`, check `name()` to find the target
- Match both `.exe` (Windows) and bare `mihomo` (Linux) -- zip may contain either
- Exclude entries with `/` or `.` to skip directories and non-binary files
- `zip 2.x` is sufficient; `zip 8.x` is available but pulls in more deps

## Links

- [018-flate2-gzip-decompression](./018-flate2-gzip-decompression.md) -- Linux .gz equivalent
