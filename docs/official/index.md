---
layout: home

hero:
  name: "FERRISGRID"
  text: "Visual control for local agents"
  tagline: "Turn screens into coordinates, and coordinates into action."
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
    - theme: alt
      text: "GitHub"
      link: https://github.com/BrunoV21/FerrisGrid-CLI
    - theme: alt
      text: "TypeScript npm mirror"
      link: https://github.com/BrunoV21/FerrisGrid-CLI-ts

features:
  - title: "Single-step execution"
    details: "Every invocation observes once or executes one validated action, then exits. The agent owns the reasoning loop."
  - title: "Coordinate-first screenshots"
    details: "Screenshots include deterministic metadata so image-space choices map back to native screen coordinates."
  - title: "Eyes plus a map"
    details: "FerrisGrid turns desktop pixels into structured observations an LLM can reason over without hiding the underlying screenshot."
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

## The idea

Computers already show the state an agent needs: pixels, windows, buttons, text fields, menus. FerrisGrid makes that state actionable by pairing screenshots with deterministic coordinates, constrained actions, and local traces.

FerrisGrid gives the machine eyes, gives the model a map, and lets Rust move fast.

## Install

```bash
cargo install ferrisgrid-cli
ferrisgrid doctor
```

An equivalent TypeScript npm package is available from [`BrunoV21/FerrisGrid-CLI-ts`](https://github.com/BrunoV21/FerrisGrid-CLI-ts):

```bash
npm install -g ferrisgrid-cli
ferrisgrid doctor
```

Feature requests and protocol changes belong in [`BrunoV21/FerrisGrid-CLI`](https://github.com/BrunoV21/FerrisGrid-CLI).

## First observe/action loop

```bash
ferrisgrid observe
```

The agent reads the returned screenshot path and coordinate metadata, decides one action, then calls:

```bash
ferrisgrid act --file .ferrisgrid/action.md
```

## Development from source

```bash
git clone https://github.com/BrunoV21/FerrisGrid-CLI.git
cd FerrisGrid-CLI
cargo build
cargo test --workspace
cargo run -q -p ferrisgrid-cli -- doctor
```

## Next steps

1. Read the [getting started guide](./getting-started/).
2. Learn the [command surface](./commands/).
3. Run FerrisGrid inside a [Docker Linux workspace](./workspaces/docker.md).
4. Open [issues and feature requests](./community.md) on GitHub.
