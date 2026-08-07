# Local Traces

FerrisGrid writes artifacts under `.ferrisgrid/` by default.

Trace data includes:

- screenshots
- metadata files
- action requests
- parsed action summaries
- action results
- human demonstration summaries and reusable action sequences
- smart pre/post demonstration checkpoints
- replay manifests and replayed action files
- errors
- recap exports

Use `--output-dir` or `FERRISGRID_OUTPUT_DIR` to redirect traces.

## Why traces matter

Local traces make agent workflows reviewable. A human can inspect the screenshots and action files that led to a result, then generate recap artifacts from the same session.

## Demonstration sessions

A recording session adds:

```text
sessions/<session_id>/
  manifest.md
  events.md
  frames/<frame_number>/
  actions/<action_number>.md
  sequences/recording.md
  sequences/sequence.md
```

`sequence.md` contains screen fingerprints, logical dimensions, event timestamps,
checkpoint frame references, and self-contained FerrisGrid action blocks. Frame and
action counters are independent because grouped typing does not produce a screenshot
per character.

The default `--text-mode redacted` removes typed payloads from the sequence and makes
it non-replayable. It does not hide text already visible in screenshots. Use
`--text-mode plain` only when storing typed content is intentional, or
`--text-mode off` to omit typed actions entirely.

Live replay never mutates the source recording. It writes a new session containing its
own manifest, checkpoint frames, action files, and event log.

## Cleanup

Use `clear` when you intentionally want to remove the output directory:

```bash
ferrisgrid clear --force
```
