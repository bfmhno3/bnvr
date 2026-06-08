# BNVR Learning Notes

Atomic notes for learning Rust through building BNVR.

## Rules

- One note = one concept. If you can split it, you should.
- No mega-notes. If a note exceeds ~30 lines of prose, split it.
- Code blocks must compile or be clearly marked as pseudocode.
- Chinese or English, your call. Mix freely. Code stays English.

## File Naming

`<number>-<kebab-case-slug>.md`

Example: `001-clap-derive-macros.md`

Numbers give ordering, slugs give meaning.

## Frontmatter

```yaml
---
id: 001
title: "Clap Derive Macros"
tags: [rust, cli, clap]
phase: 1
created: 2026-06-08
---
```

## Body Structure

```markdown
## What

One paragraph. What is this thing.

## Why

Why does BNVR need it. Or why does it matter for this project.

## How

The actual content -- code examples, explanations, gotchas.

## Links

- [002-tokio-async-runtime](./002-tokio-async-runtime.md)
- [003-serde-json-serialization](./003-serde-json-serialization.md)
```

## Links

Use relative paths: `[title](./002-slug.md)`

Broken links are fine -- they mark notes you should write later.

## Phase Tag

The `phase` field ties the note to the TODO phase. Makes it easy to find "what did I learn in Phase 1."
