# Commands

FerrisGrid exposes a small command surface. Each command is bounded and agent-friendly.

| Command | Purpose |
| --- | --- |
| `observe` | Capture screens and print compact Markdown with coordinates and paths. |
| `act` | Parse one action, validate it, execute it, capture the result, and print Markdown. |
| `doctor` | Report backend, permission, screen, and tool availability. |
| `recap` | Generate review artifacts from an existing session. |
| `clear` | Remove the output directory after explicit validation. |

The normal agent loop is `observe`, choose one action externally, then `act`.
