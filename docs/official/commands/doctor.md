# doctor

`doctor` reports whether the current environment can capture screens and emit input.

```bash
ferrisgrid doctor
```

## Options

| Option | Purpose |
| --- | --- |
| `--output-dir <path>` | Create/check a specific FerrisGrid output directory. Defaults to `.ferrisgrid`. |
| `--backend <name>` | Select capture/input backends. |

It prints:

- OS
- capture backend status
- input backend capabilities
- output directory
- discovered screens
- ffmpeg availability

Run this before an agent workflow, especially after changing permissions, backends, displays, or Docker workspace settings.

On Windows, `doctor --backend native-windows` verifies display discovery and reports mouse/keyboard capability. Run it from an unlocked interactive desktop. Input to an elevated application or the secure desktop can still be rejected at execution time by Windows integrity isolation.
