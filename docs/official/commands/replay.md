# replay

`replay` validates a complete recorded sequence. It is read-only by default and emits
OS input only when `--execute` is explicit.

```bash
ferrisgrid replay .ferrisgrid/sessions/onboarding-demo
```

The source may be either a session directory or a direct path to
`sequences/sequence.md`.

## Preflight and execution

Before the first live action, FerrisGrid parses every step, enforces the action-count
limit, rejects redacted or omitted typed content, maps every recorded screen, validates
coordinates and selected-backend input capabilities, and applies the normal action
policy. A failure leaves the desktop untouched.

```bash
ferrisgrid replay .ferrisgrid/sessions/onboarding-demo --execute
```

Live replay uses a fixed 300 ms delay by default and writes a new replay session. It
never adds files to or changes the source recording. Checkpoint screenshots are taken
after the corresponding actions.

Clipboard paste hotkeys may be valid but depend on external clipboard state. Dry-run
and live output report how many such warnings were found; FerrisGrid does not restore
or inject the recorded clipboard contents.

## Screen mapping

FerrisGrid first matches a recorded display fingerprint, then the recorded screen ID.
If the layout changed, map the old ID to a current ID explicitly:

```bash
ferrisgrid replay sequences/sequence.md \
  --map-screen screen-2=screen-1
```

Repeat `--map-screen` for multiple displays. Run `ferrisgrid doctor` or
`ferrisgrid observe` to inspect current IDs.

## Options

| Option | Purpose |
| --- | --- |
| `--execute` | Emit OS input after successful whole-sequence preflight. |
| `--delay-ms <0..30000>` | Fixed delay between actions. Defaults to `300`. |
| `--max-actions <1..1000>` | Guarded action limit. Defaults to `25`. |
| `--map-screen <recorded=current>` | Map a recorded display to a current screen. Repeatable. |
| `--output-dir <path>` | Root for a new live replay session. Defaults to `.ferrisgrid`. |
| `--session <name-or-path>` | Name the new live replay session; existing sessions are never overwritten. |
| `--backend <name>` | Select current capture/input backends. |
| `--format jpg|png` | Live checkpoint format. Defaults to `jpg`. |
| `--grid-overlay true|false` | Overlay live replay checkpoints. Defaults to `false`. |
| `--resolution fast|balanced|detail|native` | Select live checkpoint dimensions. |
| `--max-image-edge <px>` | Set an exact longest-edge cap. |
| `--no-downsample` | Keep native screenshot dimensions. |

Treat `--execute` as a privileged operation: focus the intended application, verify
the dry-run result, and be ready to interrupt the process.
