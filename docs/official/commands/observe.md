# observe

`observe` captures the current screen state and returns compact Markdown.

```bash
cargo run -q -p ferrisgrid-cli -- observe
```

## Options

| Option | Purpose |
| --- | --- |
| `--screen-id <id>` | Capture one screen instead of all screens. |
| `--output-dir <path>` | Write session data somewhere other than `.ferrisgrid`. |
| `--session <name>` | Continue or create a named session. |
| `--format jpg|png` | Choose screenshot format. |
| `--grid-overlay false` | Disable visual grid stamping when supported. |
| `--max-image-edge <px>` | Bound screenshot size sent to the agent. |
| `--no-downsample` | Keep native image dimensions. |
| `--backend <name>` | Select a capture backend. |

## Output contract

The output includes screenshot paths immediately after capture, plus dimensions and coordinate metadata for each screen.
