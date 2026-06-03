<div align="center">
  <img src="https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/refs/heads/main/docs/branding/assets/ferrisgrid-banner.png" alt="FerrisGrid - terminal-first visual control for local AI agents" width="100%" />

  <p><strong>Turn screens into coordinates, and coordinates into action.</strong></p>

  <p>
    <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-8A2BE2?style=for-the-badge">
    <img alt="Rust 2024" src="https://img.shields.io/badge/rust-2024-00E5FF?style=for-the-badge&logo=rust&logoColor=white">
    <img alt="Cargo workspace" src="https://img.shields.io/badge/cargo-workspace-A970FF?style=for-the-badge&logo=rust&logoColor=white">
    <img alt="Platforms: macOS, Linux, Windows" src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-111111?style=for-the-badge">
    <img alt="Docker workspace" src="https://img.shields.io/badge/docker-workspace-2496ED?style=for-the-badge&logo=docker&logoColor=white">
  </p>
</div>

FerrisGrid captures the current screen, maps it to deterministic coordinates, returns compact Markdown to an agent, executes one constrained action, captures the result, and exits. The agent does the reasoning. FerrisGrid handles the screen, coordinates, input, and local trace.

```text
┌──────────────┐    observe     ┌──────────────┐
│ Agent / LLM  │ ─────────────> │ FerrisGrid   │
│              │ <───────────── │ screenshot + │
│ choose one   │   Markdown     │ coordinates  │
│ action       │                └──────────────┘
│              │      act       ┌──────────────┐
│              │ ─────────────> │ validate +   │
│              │ <───────────── │ execute one  │
└──────────────┘   screenshot   └──────────────┘
```

## Why FerrisGrid?

- **Eyes plus a map:** screenshots become coordinate-backed observations an LLM can reason over.
- **Single-step by default:** every call performs one observation or one action.
- **Deterministic coordinates:** screenshots map cleanly back to native screen pixels.
- **Local-first traces:** screenshots, metadata, action requests, and results stay under `.ferrisgrid/`.
- **Cross-platform shape:** the agent-facing protocol is the same across macOS, Linux, and Windows where the platform allows it.
- **Container-friendly:** run a Linux desktop workspace in Docker and watch it through noVNC while the agent works away from your main screen.

## Installation

For normal use, install the published CLI with Cargo:

```bash
cargo install ferrisgrid-cli
ferrisgrid doctor
```

The package is [`ferrisgrid-cli`](https://crates.io/crates/ferrisgrid-cli) on crates.io and installs the `ferrisgrid` command.

## Quick Start

Capture the current screen:

```bash
ferrisgrid observe
```

Run one action from a Markdown action file:

```bash
mkdir -p .ferrisgrid
cat > .ferrisgrid/action.md <<'EOF'
status: action
action: click
screen_id: screen-1
x: 500
y: 500
button: left
wait_after_ms: 500
EOF

ferrisgrid act --file .ferrisgrid/action.md
```

## Development from source

Use a local checkout when you want to build, test, or modify FerrisGrid:

```bash
git clone https://github.com/BrunoV21/FerrisGrid-CLI.git
cd FerrisGrid-CLI
cargo build
cargo test --workspace
cargo run -q -p ferrisgrid-cli -- doctor
cargo run -q -p ferrisgrid-cli -- observe
```

Run one source-built action from a Markdown action file:

```bash
mkdir -p .ferrisgrid
cat > .ferrisgrid/action.md <<'EOF'
status: action
action: click
screen_id: screen-1
x: 500
y: 500
button: left
wait_after_ms: 500
EOF

cargo run -q -p ferrisgrid-cli -- act --file .ferrisgrid/action.md
```

## Agent Skills

If you are an agent, use this script to download and install the FerrisGrid skills:

```bash
curl -fsSL https://raw.githubusercontent.com/BrunoV21/FerrisGrid-CLI/main/scripts/install-ferrisgrid-skills.sh | sh
```

Run it from the directory that should receive the skill folders. The script downloads the repository zip, extracts the contents of `.agents/skills`, and installs those skill directories into the current directory.

## Docker Workspace

FerrisGrid can run inside a Linux container with its own X11 display. The agent calls FerrisGrid with `docker exec`; input happens inside the container, not on your main desktop.

```bash
docker build -f docker/linux-workspace.Dockerfile -t ferrisgrid-linux-workspace .

docker run --rm -d \
  --name ferrisgrid-workspace \
  -p 6080:6080 \
  -v "$PWD:/workspace" \
  ferrisgrid-linux-workspace
```

Open the viewer:

```text
http://127.0.0.1:6080/vnc.html?autoconnect=1&resize=scale
```

Then run:

```bash
docker exec ferrisgrid-workspace ferrisgrid doctor
docker exec ferrisgrid-workspace ferrisgrid observe
```

## Documentation

Official docs live in [`docs/official`](docs/official).
Brand positioning and story notes live in [`docs/branding`](docs/branding).

```bash
cd docs/official
npm install
npm run docs:dev
```

The docs use a terminal-brutalist **Terminal Violet** palette: black surfaces, violet brand/action states, cyan coordinate accents, and status colors for execution feedback.

## Project Status

FerrisGrid is early, local-first infrastructure for agent-facing visual control. The current focus is reliable observe/act behavior, local traces, recap output, and containerized Linux workspaces.

## License

MIT
