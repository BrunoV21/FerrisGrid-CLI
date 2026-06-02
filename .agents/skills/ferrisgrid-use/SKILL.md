---
name: ferrisgrid-use
description: Use FerrisGrid as an agent-facing visual control tool. Use when an agent needs to observe screens, read screenshot paths, choose normalized coordinates, execute one action, and continue by alternating observe and act calls.
---

# FerrisGrid Use

FerrisGrid is single-step. Do one observe or one action per call, then stop and inspect the Markdown result.

## Observe

Capture all screens by default:

```bash
cargo run -q -p ferrisgrid-cli -- observe
```

Capture one screen only:

```bash
cargo run -q -p ferrisgrid-cli -- observe --screen-id screen-1
```

Use smaller images for faster LLM vision:

```bash
cargo run -q -p ferrisgrid-cli -- observe --max-image-edge 1280
```

## Coordinates

- Use `coordinate_mode: normalized-1000`.
- `x=0 y=0` is the top-left of the target screen.
- `x=1000 y=1000` is the bottom-right.
- Coordinates are screen-local, not virtual-desktop coordinates.
- If multiple screens are listed, include `screen_id`.
- Read the returned `screenshot=` path and optional `metadata=` path after every call.

## Act

Write compact Markdown to an action file, not JSON or prose:

```bash
# Update .ferrisgrid/action.md with one compact Markdown action, then run:
cargo run -q -p ferrisgrid-cli -- act --file .ferrisgrid/action.md
```

Example `.ferrisgrid/action.md`:

```markdown
status: action
action: click
screen_id: screen-1
x: 500
y: 500
button: left
wait_after_ms: 500
```

Use `wait_after_ms` on UI-changing actions so FerrisGrid captures the updated UI after the action settles. It waits after executing the action and before taking the returned screenshot. Omit it for pointer moves or actions where an immediate capture is useful. Maximum: `30000`.

Suggested `wait_after_ms` values:

- `150-300`: simple desktop focus changes, opening menus, toggles, hover-triggered UI.
- `300-700`: local app tab switches, list selection, expanding rows, lightweight modals.
- `700-1200`: browser SPA route changes, message/thread switches, filtered lists, autocomplete results.
- `1200-2500`: form submissions, sign-in transitions, search results, uploads that start background work.
- `2500-5000`: full page loads, slow web dashboards, OAuth redirects, network-heavy state changes.
- `5000-15000`: installers, app launches, downloads, operations that visibly show a progress state.

Useful actions:

```markdown
status: action
action: click
screen_id: screen-1
x: 742
y: 611
button: left
wait_after_ms: 700
```

```markdown
status: action
action: scroll
screen_id: screen-1
x: 500
y: 500
delta_y: -720
wait_after_ms: 300
```

```markdown
status: action
action: type
text: hello
```

```markdown
status: action
action: press_key
key: enter
wait_after_ms: 1200
```

## Browser Tips

- If a page does not scroll, first click inside the page content, then retry scroll from that area.
- Browser overlays, galleries, and find boxes can capture input; close them before continuing.
- For product pages, `cmd+f` with a price symbol or keyword can jump to hidden details faster than repeated scrolling. Close find with `escape` before reading the final view.

## Troubleshooting

- If `observe` works but `act` through stdin returns `CoreGraphics returned no displays`, use the default `--file .ferrisgrid/action.md` path.

## Loop

1. Run `observe`.
2. Inspect returned screenshots.
3. Write exactly one action to `.ferrisgrid/action.md` and run `act --file`, adding `wait_after_ms` when the action should change visible UI.
4. Inspect the post-action screenshot path.
5. Repeat until done.

Finish with:

```markdown
status: done
reason: task complete
```
