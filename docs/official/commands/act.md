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
| `--session <name>` | Use a named session. |
| `--format jpg|png` | Choose post-action screenshot format. |
| `--resolution fast|balanced|detail|native` | Use a named screenshot-size preset. `balanced` is the adaptive default. |
| `--max-image-edge <px>` | Use a fixed longest-edge cap instead of the adaptive default. |
| `--no-downsample` | Keep native image dimensions. |
| `--backend <name>` | Select capture/input backend. |

## Safety model

FerrisGrid validates action type, fields, coordinates, target screen, and policy before emitting input. Ambiguous multi-screen actions must include `screen_id`.
