# clear

`clear` removes the FerrisGrid output directory after safety validation.

```bash
cargo run -q -p ferrisgrid-cli -- clear --force
```

## Options

| Option | Purpose |
| --- | --- |
| `--output-dir <path>` | Clear a specific output directory. Defaults to `.ferrisgrid`. |
| `--force` | Required for custom output directories and useful for explicit cleanup. |

FerrisGrid refuses to clear unsafe targets such as the current directory or filesystem root.
