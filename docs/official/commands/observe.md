# observe

`observe` captures the current screen state and returns compact Markdown.

```bash
ferrisgrid observe
```

## Options

| Option | Purpose |
| --- | --- |
| `--screen-id <id>` | Capture one screen instead of all screens. |
| `--output-dir <path>` | Write session data somewhere other than `.ferrisgrid`. |
| `--session <name>` | Continue or create a named session. |
| `--format jpg|png` | Choose screenshot format. |
| `--grid-overlay false` | Disable visual grid stamping when supported. |
| `--resolution fast|balanced|detail|native` | Use a named screenshot-size preset. `balanced` is the adaptive default. |
| `--max-image-edge <px>` | Use a fixed longest-edge cap instead of the adaptive default. |
| `--no-downsample` | Keep native image dimensions. |
| `--backend <name>` | Select a capture backend. |

## Output contract

The output includes screenshot paths immediately after capture, the `image_size_limit` used for downsampling, plus dimensions and coordinate metadata for each screen.

`balanced` keeps at least an 800 px long edge and raises that cap on wide screens to preserve roughly a 500 px short edge. FerrisGrid never upscales screenshots.
