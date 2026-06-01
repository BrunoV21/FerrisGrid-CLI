# FerrisGrid Architecture

**Product name:** FerrisGrid
**Document type:** Architecture and documentation foundation
**Version:** 0.1
**Date:** 2026-06-01
**Status:** Draft

---

## 1. Purpose

This document defines the foundation for FerrisGrid implementation and documentation.

It serves two audiences:

- **Internal maintainers** who need a stable architecture for crates, protocols, storage, testing, and cross-platform behavior.
- **User-facing documentation authors** who need a clear information architecture for agent developers, researchers, and humans reviewing sessions.

The product contract comes from [docs/prd.md](./prd.md): FerrisGrid is a local, single-step visual computer control tool for agents. The agent calls FerrisGrid to observe screens, receives compact Markdown with local screenshot paths and coordinate metadata, then calls FerrisGrid again with one constrained action.

FerrisGrid must not be designed as a long-running autonomous runner for normal task execution.

---

## 2. Architectural Principles

1. **Single-step by default**
   Each invocation performs one observation or one action, writes local trace data, returns compact Markdown, and exits.

2. **Agent owns reasoning**
   FerrisGrid does not plan a task, call an LLM provider in MVP, or decide the next action. The external agent runtime analyzes screenshots and calls FerrisGrid with the next action.

3. **Multi-screen first**
   `observe` captures all screens by default. `screen_id` can narrow observation or action to a specific screen. Ambiguous multi-screen actions must fail with a clear Markdown error.

4. **Local traceability**
   Screenshots, metadata, action requests, parsed actions, results, metrics, and recap artifacts are written under `.ferrisgrid/` unless configured otherwise.

5. **Compact Markdown interface**
   External tool output is compact Markdown, not JSON. Internal Rust types may be structured, but the agent-facing protocol is Markdown.

6. **Coordinate correctness before speed**
   Every coordinate returned to or accepted from an agent must map deterministically between agent image space and native screen space.

7. **Policy-gated execution**
   Every action passes validation and policy checks before input is emitted to the OS.

---

## 3. System Context

```text
Human task
  |
  v
External agent runtime / LLM
  |
  | calls
  v
FerrisGrid CLI or local tool API
  |
  +--> observe: capture screen(s), write screenshots, return Markdown
  |
  +--> act: parse one action, validate, execute, capture result, return Markdown
  |
  +--> recap: human-facing recap/video generation from local session files
```

Normal task execution is:

```text
agent calls observe -> agent analyzes screenshots -> agent calls act -> agent calls observe or act again
```

FerrisGrid should be safe to call repeatedly from an agent runtime because each command has a narrow, bounded responsibility.

---

## 4. Command Surface

### 4.1 Agent-Facing MVP Commands

#### `ferrisgrid observe`

Captures current screen state and returns compact Markdown.

Default behavior:

- Captures all available screens.
- Writes one screenshot and metadata file per screen.
- Returns `screen_id`, image dimensions, native dimensions, coordinate mode, and screenshot path per screen.
- Exits after the observation is written.

Optional behavior:

- `--screen-id <id>` captures only one screen.
- `--grid-overlay true` writes grid-overlay screenshots for coordinate debugging.

#### `ferrisgrid act`

Executes exactly one constrained action and returns compact Markdown.

Default behavior:

- Parses one Markdown action block.
- Validates action type, fields, coordinates, `screen_id`, and policy.
- Emits at most one OS input action.
- Captures the post-action screen state.
- Returns action result and latest screenshot path.
- Exits after the action attempt.

Multi-screen behavior:

- `screen_id` is required when the active context has multiple screens.
- Missing `screen_id` in an ambiguous context returns an ambiguity error.
- Coordinates are local to the target screen unless virtual-desktop coordinates are explicitly supported.

### 4.2 Human-Facing MVP Command

#### `ferrisgrid recap <session_path>`

Generates human-review artifacts from existing local session data.

Outputs:

- `export/recap.md`
- optional video artifact such as `export/session.mp4`
- optional GIF or frame sequence fallback

### 4.3 Roadmap Command

#### `ferrisgrid record`

Records a human demonstration and writes a reusable action sequence.

Expected output:

- `sequences/recording.md`
- `sequences/sequence.md`
- pre-action and post-action screenshots where practical

This is for authoring reproducible workflows. It is not the normal agent execution path.

---

## 5. Internal Crate Architecture

Recommended Rust workspace layout:

```text
crates/
  ferrisgrid-cli/
  ferrisgrid-core/
  ferrisgrid-capture/
  ferrisgrid-input/
  ferrisgrid-agent/
  ferrisgrid-record/
  ferrisgrid-export/
```

### 5.1 `ferrisgrid-cli`

Responsibilities:

- Parse CLI commands and flags.
- Load config and resolve session paths.
- Call core services.
- Print compact Markdown responses.
- Handle interrupts and map errors to Markdown.

### 5.2 `ferrisgrid-core`

Responsibilities:

- Shared domain types.
- Session lifecycle.
- Single-step orchestration.
- Config model.
- Error model.
- Policy gate coordination.

The core should expose operations similar to:

```rust
observe(request) -> ObservationResult
act(request) -> ActionResult
recap(request) -> RecapResult
```

These operations should be usable from the CLI and future library/API integrations.

### 5.3 `ferrisgrid-capture`

Responsibilities:

- Screen discovery.
- Stable session-local `screen_id` assignment.
- Screenshot capture.
- Optional image resizing, compression, and grid overlay.
- Screen metadata collection.
- Coordinate-space metadata.

Important invariant:

Every captured image must have metadata sufficient to map agent coordinates back to native screen coordinates.

### 5.4 `ferrisgrid-input`

Responsibilities:

- Native input execution.
- Mouse actions.
- Keyboard actions.
- Platform permission checks.
- Backend capability reporting.

This crate must never execute an action that has not passed core validation and policy checks.

### 5.5 `ferrisgrid-agent`

Responsibilities:

- Parse compact Markdown action blocks.
- Validate protocol shape before semantic validation.
- Normalize action fields into internal action types.
- Produce compact Markdown ambiguity and parse errors.

This crate does not call LLM providers.

### 5.6 `ferrisgrid-record`

Responsibilities:

- Session manifest.
- Events log.
- Metrics log.
- Action traces.
- Human demonstration recording.
- Reproducible sequence logs.

### 5.7 `ferrisgrid-export`

Responsibilities:

- Recap generation.
- Frame sequence export.
- Optional cursor/action overlays.
- Optional FFmpeg integration.
- GIF or image-sequence fallback.

---

## 6. Runtime Flow

### 6.1 Observation Flow

```text
CLI parse
  -> config load
  -> session create/resume
  -> screen discovery
  -> capture all screens or selected screen
  -> image processing
  -> metadata write
  -> event/metrics write
  -> compact Markdown response
  -> exit
```

Required response shape:

```md
## FerrisGrid Observation
- session: .ferrisgrid/sessions/<session_id>
- step: 1
- coordinate_mode: normalized-1000
- screens: 2
- screen: screen-1 primary=true image=1280x832 native=3024x1964 screenshot=.ferrisgrid/sessions/<session_id>/frames/000001/screen-1.jpg
- screen: screen-2 primary=false image=1280x720 native=2560x1440 screenshot=.ferrisgrid/sessions/<session_id>/frames/000001/screen-2.jpg
```

### 6.2 Action Flow

```text
CLI parse
  -> config load
  -> session resume
  -> parse compact Markdown action
  -> validate action fields
  -> validate screen_id and coordinate bounds
  -> policy gates
  -> execute one OS input action
  -> capture post-action screen
  -> write action/result/events/metrics
  -> compact Markdown response
  -> exit
```

Required response shape:

```md
## FerrisGrid Action Result
- session: .ferrisgrid/sessions/<session_id>
- step: 2
- action: click screen_id=screen-1 x=742 y=611 button=left
- result: success
- screenshot: .ferrisgrid/sessions/<session_id>/frames/000002/screen-1.jpg
```

### 6.3 Ambiguous Multi-Screen Action Flow

If multiple screens are available and the agent omits `screen_id`, FerrisGrid must not guess.

Required response shape:

```md
## FerrisGrid Action Error
- type: ambiguous_screen
- result: rejected
- reason: screen_id is required because multiple screens are active
- available_screen: screen-1 screenshot=.ferrisgrid/sessions/<session_id>/frames/000001/screen-1.jpg
- available_screen: screen-2 screenshot=.ferrisgrid/sessions/<session_id>/frames/000001/screen-2.jpg
```

---

## 7. Data Model

### 7.1 Screen

Core fields:

- `screen_id`
- `name`
- `is_primary`
- `origin_x`
- `origin_y`
- `native_width`
- `native_height`
- `scale_factor`
- `coordinate_mode`
- `screenshot_path`
- `metadata_path`

### 7.2 Captured Frame

A frame is a step-level directory containing one or more screen captures.

```text
frames/
  000001/
    screen-1.jpg
    screen-1.meta.md
    screen-2.jpg
    screen-2.meta.md
```

### 7.3 Agent Action

MVP action fields:

- `status`
- `action`
- `screen_id`
- coordinate fields such as `x`, `y`, `from_x`, `from_y`, `to_x`, `to_y`
- action-specific fields such as `button`, `text`, `key`, `keys`, `delta_y`
- `confidence`
- `reason`

`screen_id` is optional only when the active context is unambiguous.

### 7.4 Session

Recommended session structure:

```text
.ferrisgrid/
  config.toml
  sessions/
    <session_id>/
      manifest.md
      config.snapshot.toml
      frames/
      agent/
      actions/
      sequences/
      events.md
      metrics.md
      export/
```

---

## 8. Coordinate Mapping

Default coordinate mode:

```text
normalized-1000
```

Agent coordinates are screen-local:

```text
agent_x: 0..1000
agent_y: 0..1000
```

Mapping:

```text
image_x = round((agent_x / 1000) * image_width)
image_y = round((agent_y / 1000) * image_height)
native_x = screen_origin_x + round((image_x / image_width) * native_width)
native_y = screen_origin_y + round((image_y / image_height) * native_height)
```

Validation rules:

- Coordinates must be numeric.
- Coordinates must be in bounds for the selected screen.
- `screen_id` must exist.
- Display topology must not have changed unexpectedly.
- If topology changed, reject stale actions and return a fresh observation/error response.

---

## 9. Safety Architecture

All actions pass through this chain:

```text
parse -> semantic validation -> coordinate validation -> screen validation -> policy gate -> execution
```

MVP policy gates:

- reject unknown actions
- reject missing `screen_id` in multi-screen contexts
- reject out-of-bounds coordinates
- cap text input length
- cap scroll deltas
- cap drag duration
- redact typed text in logs by default
- block or flag risky form submission actions when detectable

FerrisGrid should return policy results to the agent in compact Markdown instead of prompting the human during the action sequence.

---

## 10. Cross-Platform Boundaries

FerrisGrid should hide platform-specific implementation details behind traits:

```rust
trait CaptureBackend {
    fn list_screens(&self) -> Result<Vec<ScreenInfo>>;
    fn capture(&self, target: CaptureTarget) -> Result<CapturedFrame>;
}

trait InputBackend {
    fn capabilities(&self) -> InputCapabilities;
    fn execute(&self, action: NativeAction) -> Result<ActionResult>;
}

trait PermissionBackend {
    fn check_permissions(&self) -> Result<PermissionReport>;
}
```

Platform concerns:

- **Windows:** DPI scaling, elevated-window input restrictions, native input APIs.
- **macOS:** Screen Recording permission, Accessibility permission, Retina coordinate conversion.
- **Linux X11:** direct capture/input paths where available.
- **Linux Wayland:** compositor restrictions, portals, PipeWire, user approval flows.

---

## 11. Documentation Architecture

FerrisGrid documentation should be split by audience and purpose. User-facing docs live under `docs/user-facing/`. Internal, product, roadmap, and implementation docs stay directly under `docs/`.

### 11.1 Internal Documentation

Internal docs should help maintainers build and change FerrisGrid safely.

Recommended structure:

```text
docs/
  prd.md
  architecture.md
  crates.md
  protocol.md
  coordinate-mapping.md
  storage-format.md
  platform-backends.md
  policy-gates.md
  testing.md
  release-checklist.md
  record.md
  sequence-reproduction.md
  video-export.md
  plugins.md
```

Required internal docs:

- **Crates:** ownership boundaries and dependency direction.
- **Protocol:** compact Markdown observe/action/recap formats.
- **Coordinate mapping:** formulas, examples, golden fixtures, HiDPI notes.
- **Storage format:** session layout and file naming.
- **Platform backends:** Windows, macOS, Linux X11, Linux Wayland.
- **Policy gates:** validation, redaction, and blocked-action behavior.
- **Testing:** unit, integration, golden, and manual QA matrices.

### 11.2 User-Facing Documentation

User-facing docs should explain how external agents call FerrisGrid and how humans review sessions.

Recommended structure:

```text
docs/
  user-facing/
    overview.md
    installation.md
    quickstart.md
    agent-integration.md
    observe.md
    act.md
    recap.md
    sessions.md
    multi-screen.md
    privacy.md
    platform-permissions.md
    troubleshooting.md
```

Required user docs:

- **Overview:** FerrisGrid is an agent-facing single-step tool.
- **Installation:** install binary and run `doctor`.
- **Quickstart:** show `observe`, one `act`, then `recap`.
- **Agent integration:** explain repeated calls from an external agent runtime.
- **Observe:** all-screen default, selected-screen `screen_id`, screenshot paths.
- **Act:** compact Markdown action blocks, one action per call, post-action screenshots.
- **Recap:** generate Markdown/video from stored screenshots.
- **Sessions:** where files are stored and how to inspect them.
- **Multi-screen:** how `screen_id` works and why ambiguous actions are rejected.
- **Privacy:** screenshots are local, external agents may send them elsewhere.
- **Platform permissions:** macOS, Windows, Linux X11/Wayland notes.
- **Troubleshooting:** missing permissions, stale screen topology, ambiguous screen errors.

### 11.3 Roadmap Documentation

Roadmap docs should cover features not required for MVP:

```text
docs/
  record.md
  sequence-reproduction.md
  video-export.md
  plugins.md
```

`record.md` should explain demonstration recording:

- human performs workflow
- FerrisGrid records actions and coordinates
- screenshots are captured around actions
- `sequence.md` is created
- future reproduction uses the same action protocol as `act`

---

## 12. Documentation Style Rules

All FerrisGrid docs should follow these rules:

- Prefer concise Markdown examples.
- Avoid JSON for user-facing command output.
- Always show screenshot paths after observe/action examples.
- Say clearly when a command is agent-facing versus human-facing.
- Do not describe `observe` or `act` as long-running task runners.
- Treat `screen_id` as first-class in multi-screen examples.
- Show failure examples for ambiguous multi-screen actions.
- Keep platform-specific caveats concrete and actionable.

---

## 13. Initial Documentation Backlog

1. Split compact Markdown protocol details out of PRD into `docs/protocol.md`.
2. Add `docs/user-facing/quickstart.md` with one `observe`, one `act`, and one `recap`.
3. Add `docs/user-facing/multi-screen.md` focused on `screen_id` behavior.
4. Add `docs/coordinate-mapping.md` with formulas and golden fixtures.
5. Add `docs/storage-format.md` with session layout and naming rules.
6. Add `docs/record.md` for demonstration recording and `sequence.md`.
7. Add `docs/user-facing/privacy.md` explaining local screenshots and external agent risk.

---

## 14. Implementation Readiness Checklist

- `observe` can capture all screens and return screenshot paths.
- `observe --screen-id <id>` captures only one screen.
- `act` executes exactly one action and exits.
- `act` rejects ambiguous multi-screen actions.
- Every response is compact Markdown.
- Every observe/action response includes screenshot paths.
- Session storage matches the documented layout.
- Coordinate mapping has golden tests.
- Platform permission failures are actionable.
- Human `recap` can read existing session files without an LLM.
- Roadmap `record` has a documented sequence format before implementation starts.
