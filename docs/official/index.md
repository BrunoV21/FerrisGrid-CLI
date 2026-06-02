---
layout: home

hero:
  name: "FERRISGRID"
  text: "Visual control for local agents"
  tagline: "Capture screens. Return deterministic coordinates. Execute exactly one constrained action. Keep every screenshot and action trace on your machine."
  actions:
    - theme: brand
      text: "Get Started"
      link: /getting-started/
    - theme: alt
      text: "Commands"
      link: /commands/
    - theme: alt
      text: "Docker Workspace"
      link: /workspaces/docker
    - theme: alt
      text: "Architecture"
      link: /concepts/architecture

features:
  - title: "Single-step execution"
    details: "Every invocation observes once or executes one validated action, then exits. The agent owns the reasoning loop."
  - title: "Coordinate-first screenshots"
    details: "Screenshots include deterministic metadata so image-space choices map back to native screen coordinates."
  - title: "Local traces"
    details: "Screenshots, metadata, action requests, parsed actions, results, and recap artifacts stay under .ferrisgrid by default."
  - title: "Docker workspace"
    details: "Run a Linux desktop in the background, watch it through noVNC, and keep agent input away from your main desktop."
  - title: "Compact Markdown protocol"
    details: "FerrisGrid prints agent-readable Markdown for observations, actions, errors, and doctor checks."
  - title: "Cross-platform shape"
    details: "The same observe/act interface is designed to span macOS, Linux, and Windows as platform backends mature."
---

<div class="fg-terminal">
<strong>$ ferrisgrid observe</strong><br>
## FerrisGrid Observation<br>
- session: .ferrisgrid/session-...<br>
- step: 1<br>
- coordinate_mode: normalized-1000<br>
- screen_id: screen-1<br>
- screenshot: <span class="grid">.ferrisgrid/.../screen-1.png</span><br>
<br>
<strong>$ ferrisgrid act --file .ferrisgrid/action.md</strong><br>
<span class="ok">status: executed</span>
</div>

## Install from source

```bash
cargo build
cargo run -q -p ferrisgrid-cli -- doctor
```

## First observe/action loop

```bash
cargo run -q -p ferrisgrid-cli -- observe
```

The agent reads the returned screenshot path and coordinate metadata, decides one action, then calls:

```bash
cargo run -q -p ferrisgrid-cli -- act --file .ferrisgrid/action.md
```

## Next steps

1. Read the [getting started guide](./getting-started/).
2. Learn the [command surface](./commands/).
3. Run FerrisGrid inside a [Docker Linux workspace](./workspaces/docker.md).
