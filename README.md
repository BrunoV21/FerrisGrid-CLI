# FerrisGrid

**Single-step visual computer control for local agents.**

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

- **Single-step by default:** every call performs one observation or one action.
- **Deterministic coordinates:** screenshots map cleanly back to native screen pixels.
- **Local-first traces:** screenshots, metadata, action requests, and results stay under `.ferrisgrid/`.
- **Cross-platform shape:** the agent-facing protocol is the same across macOS, Linux, and Windows where the platform allows it.
- **Container-friendly:** run a Linux desktop workspace in Docker and watch it through noVNC while the agent works away from your main screen.

## Quick Start

Build and check the CLI from source:

```bash
cargo build
cargo run -q -p ferrisgrid-cli -- doctor
```

Capture the current screen:

```bash
cargo run -q -p ferrisgrid-cli -- observe
```

Run one action from a Markdown action file:

```bash
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
