---
name: ferrisgrid-install
description: Install and verify the published FerrisGrid CLI from crates.io. Use when setting up FerrisGrid, checking prerequisites, installing the ferrisgrid command, or confirming screen/input permissions.
---

# FerrisGrid Install

Use this when a machine needs the normal published FerrisGrid CLI. Only use a local checkout when the user explicitly asks for source development.

## Steps

1. Check Rust is available:

```bash
cargo --version
```

2. Install or update the published CLI from crates.io:

```bash
cargo install ferrisgrid-cli
```

3. Confirm the installed binary is available:

```bash
ferrisgrid doctor
```

4. Smoke-test capture without touching the real desktop:

```bash
ferrisgrid observe --backend fake
```

## Development from source

Use these commands only inside a local FerrisGrid checkout when modifying the project:

```bash
cargo build -p ferrisgrid-cli
cargo test --workspace
cargo run -q -p ferrisgrid-cli -- doctor
```

## Notes

- On macOS, real capture needs Screen Recording permission.
- Real actions need Accessibility permission.
- Default output is `.ferrisgrid/`.
- Use `--backend fake` only for protocol smoke tests; normal usage should use the native backend.
