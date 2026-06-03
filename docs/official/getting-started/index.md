# Getting Started

FerrisGrid is a local CLI engine for agent-facing visual control. A normal loop alternates between `observe` and `act`:

```text
observe -> agent inspects screenshot -> act -> observe again
```

FerrisGrid does not plan tasks or run long autonomous sequences. It gives an external agent a reliable screen primitive.

## Requirements

- Rust toolchain for Cargo installs and local builds.
- Platform permissions for screen capture and input injection.
- Docker if you want an isolated Linux desktop workspace.

## Install

```bash
cargo install ferrisgrid-cli
```

## Check the environment

```bash
ferrisgrid doctor
```

The doctor command reports OS, capture backend, input backend, output directory, screens, and ffmpeg availability.

## Capture a screen

```bash
ferrisgrid observe
```

FerrisGrid writes screenshots and metadata under `.ferrisgrid/` unless `FERRISGRID_OUTPUT_DIR` or `--output-dir` changes it.

## Development from source

```bash
git clone https://github.com/BrunoV21/FerrisGrid-CLI.git
cd FerrisGrid-CLI
cargo build
cargo test --workspace
cargo run -q -p ferrisgrid-cli -- doctor
```
