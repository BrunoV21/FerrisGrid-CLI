---
name: ferrisgrid-install
description: Install, build, and verify FerrisGrid from a local checkout. Use when setting up FerrisGrid, checking prerequisites, building the CLI, or confirming screen/input permissions.
---

# FerrisGrid Install

Use from the FerrisGrid repo root.

## Steps

1. Check Rust is available:

```bash
cargo --version
```

2. Build the CLI:

```bash
cargo build -p ferrisgrid-cli
```

3. Run tests:

```bash
cargo test
```

4. Check the local backend and permissions:

```bash
cargo run -q -p ferrisgrid-cli -- doctor
```

5. Smoke-test capture without touching the real desktop:

```bash
cargo run -q -p ferrisgrid-cli -- observe --backend fake
```

## Notes

- On macOS, real capture needs Screen Recording permission.
- Real actions need Accessibility permission.
- Default output is `.ferrisgrid/`.
- Use `--backend fake` only for protocol smoke tests; normal usage should use the native backend.
