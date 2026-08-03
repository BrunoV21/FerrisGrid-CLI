# Docker Linux Workspace

FerrisGrid can run inside a Linux container with its own X11 display. The agent calls FerrisGrid from your host terminal with `docker exec`, while mouse and keyboard actions happen only inside the container display.

Use a macOS VM when the task needs native macOS apps. Docker provides a Linux desktop workspace, not a containerized macOS session.

## Build

The Docker image installs the published `ferrisgrid-cli` package from crates.io. By default it installs the latest published version.

```bash
docker build -f docker/linux-workspace.Dockerfile -t ferrisgrid-linux-workspace .
```

Build a specific published version:

```bash
docker build \
  --build-arg FERRISGRID_VERSION=0.3.0 \
  -f docker/linux-workspace.Dockerfile \
  -t ferrisgrid-linux-workspace .
```

## Run

```bash
docker run --rm -d \
  --name ferrisgrid-workspace \
  -p 6080:6080 \
  -v "$PWD:/workspace" \
  ferrisgrid-linux-workspace
```

Open the workspace viewer:

```text
http://127.0.0.1:6080/vnc.html?autoconnect=1&resize=scale
```

## Use from the host

```bash
docker exec ferrisgrid-workspace ferrisgrid doctor
docker exec ferrisgrid-workspace ferrisgrid observe
```

Write an action file in the repo, then execute it inside the container:

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

docker exec ferrisgrid-workspace ferrisgrid act --file .ferrisgrid/action.md
```

## Backend

The image sets:

```text
DISPLAY=:99
FERRISGRID_BACKEND=native-linux-x11
FERRISGRID_OUTPUT_DIR=/workspace/.ferrisgrid
```

The Linux backend uses X11 tools:

- `xrandr` or `xdpyinfo` for screen discovery
- ImageMagick `import` for screenshots
- `xdotool` for mouse and keyboard input

## Resize

Set `XVFB_SCREEN` when starting the container:

```bash
docker run --rm -d \
  --name ferrisgrid-workspace \
  -p 6080:6080 \
  -e XVFB_SCREEN=1440x900x24 \
  -v "$PWD:/workspace" \
  ferrisgrid-linux-workspace
```
