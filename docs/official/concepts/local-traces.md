# Local Traces

FerrisGrid writes artifacts under `.ferrisgrid/` by default.

Trace data includes:

- screenshots
- metadata files
- action requests
- parsed action summaries
- action results
- errors
- recap exports

Use `--output-dir` or `FERRISGRID_OUTPUT_DIR` to redirect traces.

## Why traces matter

Local traces make agent workflows reviewable. A human can inspect the screenshots and action files that led to a result, then generate recap artifacts from the same session.

## Cleanup

Use `clear` when you intentionally want to remove the output directory:

```bash
ferrisgrid clear --force
```
