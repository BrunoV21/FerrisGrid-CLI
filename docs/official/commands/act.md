# act

`act` executes exactly one constrained action and captures the resulting screen state.

```bash
ferrisgrid act --file .ferrisgrid/action.md
```

## Action file

```yaml
status: action
action: click
screen_id: screen-1
x: 500
y: 500
button: left
wait_after_ms: 500
```

## Options

| Option | Purpose |
| --- | --- |
| `--file <path>` | Read action Markdown from a file. Without it, stdin is used. |
| `--dry-run` | Validate the action without emitting OS input. |
| `--output-dir <path>` | Read/write session data somewhere other than `.ferrisgrid`. |
| `--session <name>` | Use an existing named session. Defaults to the latest session. |
| `--screen-id <id>` | Default target for pointer actions that omit `screen_id`. |
| `--format jpg|png` | Choose post-action screenshot format. |
| `--grid-overlay true|false` | Enable or disable visual grid stamping on the post-action screenshot. |
| `--resolution fast|balanced|detail|native` | Use a named screenshot-size preset. `balanced` is the adaptive default. |
| `--max-image-edge <px>` | Use a fixed longest-edge cap instead of the adaptive default. |
| `--no-downsample` | Keep native image dimensions. |
| `--backend <name>` | Select capture/input backend. |

## Safety model

FerrisGrid validates action type, fields, coordinates, target screen, and policy before emitting input. Pointer actions on multi-screen systems must include `screen_id` in the action file or pass `--screen-id <id>` to `act`.

Non-screen actions such as `type`, `press_key`, `hotkey`, and `wait` do not require `screen_id`.

Terminal statuses such as `status: done` and `status: fail` do not execute input and do not capture a post-action screenshot; their result reports `screens: 0`.

On Windows, native actions use physical, per-monitor-DPI-aware coordinates. Run FerrisGrid and the controlled application at the same elevation level because Windows blocks synthetic input crossing into an elevated or secure desktop.
