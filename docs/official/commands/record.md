# record

`record` watches a human perform a workflow and writes semantic FerrisGrid actions plus
smart visual checkpoints. Native recording currently requires macOS 13 or newer.

```bash
ferrisgrid record --session onboarding-demo
```

## Controls

| Control | Result |
| --- | --- |
| `Control+Option+Command+Escape` | Stop and finalize the sequence. |
| `Control+Option+Command+P` | Pause or resume input observation and rolling capture. |
| Ctrl+C in the launching terminal | Stop and finalize the sequence. |

The default three-second countdown gives you time to focus the target application.
The stop and pause shortcuts are reserved by the recorder and are not added to the
sequence.

## Smart capture

FerrisGrid records clicks, double-clicks, same-screen drags, debounced scrolls,
keypresses, hotkeys, and grouped printable typing. It keeps a four-frames-per-second
rolling cache in memory, then persists an initial frame, pre/post frames around semantic
boundaries, and a final frame. Typing does not produce a screenshot per character;
Enter, navigation keys, hotkeys, clicks, completed drags, and completed scrolls create
checkpoints.

Standalone pointer movement is ignored. Cross-screen drags and native Windows/Linux
event recording are not currently supported. A display-topology change stops the
recording rather than writing ambiguous coordinates.

## Text privacy

| Mode | Behavior |
| --- | --- |
| `redacted` | Default. Keep a typed-action marker and length, but omit its content. The sequence cannot execute. |
| `plain` | Store exact typed content. Required for replaying typed actions. |
| `off` | Mark typed actions as omitted. The sequence cannot execute. |

Redaction applies to action payloads only. Screenshots may still show passwords,
messages, personal data, or other sensitive content visible on screen. Clipboard paste
hotkeys are marked as external-state dependencies; clipboard contents are never copied
into the sequence.

## Options

| Option | Purpose |
| --- | --- |
| `--output-dir <path>` | Trace root. Defaults to `.ferrisgrid`. |
| `--session <name-or-path>` | Create a named recording session; existing sessions are never overwritten. |
| `--text-mode redacted|plain|off` | Choose typed-text storage. Defaults to `redacted`. |
| `--fps <1..30>` | Rolling-cache capture rate. Defaults to `4`. |
| `--settle-ms <ms>` | Delay before a post-action checkpoint. Defaults to `300`. |
| `--countdown-ms <ms>` | Delay before event observation starts. Defaults to `3000`. |
| `--format jpg|png` | Stored checkpoint format. Defaults to `jpg`. |
| `--resolution fast|balanced|detail|native` | Select checkpoint dimensions. |
| `--max-image-edge <px>` | Set an exact longest-edge cap. |
| `--no-downsample` | Keep native screenshot dimensions. |
| `--backend native|native-macos|fake` | Select the event/capture backend. `fake` is for protocol tests. |

## Output

The result points to the session, `sequences/recording.md`, and the reusable
`sequences/sequence.md`. Action files use the same compact Markdown vocabulary as
`act`; sequence metadata adds timestamps, screen fingerprints, checkpoint reasons, and
frame references.

Run `ferrisgrid doctor` first if capture or event observation cannot start.
