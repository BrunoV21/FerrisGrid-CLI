# recap

`recap` generates human-review artifacts from an existing session path.

```bash
cargo run -q -p ferrisgrid-cli -- recap .ferrisgrid/session-id
```

## Options

| Option | Purpose |
| --- | --- |
| `--video mp4|gif` | Request a video artifact when supported. |
| `--framerate <fps>` | Set recap video frame rate. |

Recaps are built from existing traces. They are not part of the normal observe/act execution loop.
