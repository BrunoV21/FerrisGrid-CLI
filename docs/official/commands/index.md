# Commands

FerrisGrid exposes a small command surface. Each command is bounded and agent-friendly.

| Command | Purpose |
| --- | --- |
| `observe` | Capture screens and print compact Markdown with coordinates and paths. |
| `act` | Parse one action, validate it, execute it, capture the result, and print Markdown. |
| `record` | Record a human macOS demonstration as semantic actions and smart visual checkpoints. |
| `replay` | Preflight a recorded sequence and, with `--execute`, reproduce it through guarded OS input. |
| `doctor` | Report backend, permission, screen, and tool availability. |
| `recap` | Generate review artifacts from an existing session. |
| `clear` | Remove the output directory after explicit validation. |

The normal agent loop is `observe`, choose one action externally, then `act`.
`record` and `replay` are separate human demonstration-authoring tools; they do not
change the single-step agent protocol.
