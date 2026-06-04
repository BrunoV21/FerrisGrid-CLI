# recap

`recap` generates human-review artifacts from an existing session path.

```bash
ferrisgrid recap .ferrisgrid/session-id
```

## Options

| Option | Purpose |
| --- | --- |
| `--video mp4` | Request an MP4 video artifact. |
| `--framerate <fps>` | Set recap video frame rate. Defaults to 2. |
| `--fps <fps>` | Alias for `--framerate`. |

Recaps are built from existing traces. They are not part of the normal observe/act execution loop.
