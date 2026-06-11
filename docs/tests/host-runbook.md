# Host Benchmark Runbook

This runbook defines the host-only execution path for FerrisGrid comparison benchmarks. It supersedes Docker workspace setup for host benchmark runs.

Use the companion agent skill `benchmark-framework-runner` when you want Codex to execute one benchmark pass for a specific `framework` value.

## 1. Prepare Host Environment

Required baseline tools:

```bash
cargo --version
ferrisgrid help
ferrisgrid doctor
node --version
python3 --version
```

Optional competitor tools are recorded as unavailable when they are not installed:

```bash
npx playwright --version
agent-browser --help
browser-use --help
python3 -c "import selenium; print(selenium.__version__)"
```

## 2. Choose A Framework

Run the same scenario set once for each framework below:

- `ferrisgrid`
- `playwright`
- `agent-browser`
- `browser-use`
- `selenium`

The framework value determines the allowed execution surface. Do not mix surfaces inside one trial.

## 3. Start Local Fixtures

Browser fixtures live in `docs/tests/fixtures/browser/`.

Serve them from the repository root:

```bash
python3 -m http.server 4173 --directory docs/tests/fixtures/browser
```

Fixture URL:

```text
http://127.0.0.1:4173/
```

Use the `scenario` query parameter to reset and open one scenario:

```text
http://127.0.0.1:4173/?scenario=browser-button-state
```

## 4. Timed Execution Rules

For each `framework`/scenario/trial:

1. Start with a fresh scenario URL containing `reset=1`.
2. Start wall-clock timing after the task prompt is issued.
3. Use only the selected competitor surface during timed execution.
4. Stop timing when the visible success state or assertion evidence is reached.
5. Record the run with `results-template.md`.
6. Capture `started_at`, `ended_at`, and `wall_time_ms` from the session log or benchmark wrapper.
7. Capture `tokens_input`, `tokens_output`, and `tokens_total` from the Codex session ledger when the tooling exposes it. If it does not, record `pending` and explain why.

Host execution keeps the existing fairness rules:

- No external websites.
- No authenticated sessions.
- Same task prompt for every competitor.
- Same fixture reset for every trial.
- No MCP tools during timed execution.
- Do not inspect fixture source during timed execution unless the scenario explicitly allows it.

## 5. Fixture Validation

The browser fixture includes a smoke runner used to confirm the fixture itself works:

```bash
npx -y -p playwright node docs/tests/fixtures/browser/playwright-smoke.mjs http://127.0.0.1:4173/
```

This validates the benchmark targets. Treat it as fixture validation unless the run is explicitly labeled as a Playwright competitor trial.

## 6. Result Storage

Store host-only results under:

```text
docs/tests/results/host-v1/
```

Use one Markdown file per framework/scenario/trial.

## 7. FerrisGrid Session Timing

FerrisGrid session logs expose exact millisecond timestamps in:

- `.ferrisgrid/sessions/<session_id>/manifest.md`
- `.ferrisgrid/sessions/<session_id>/events.md`

Use those values to populate `started_at`, `ended_at`, and `wall_time_ms` in the result files.

## 8. Token Capture

The benchmark cares about token cost as much as wall time. When the executor can read a Codex session ledger, record the exact token counts in the result file.

If the current execution surface does not expose a per-run ledger, backfill token usage from the Codex JSONL log by intersecting the benchmark run window with the session log timestamps. Do not invent numbers.

Record the exact input, output, and total tokens for the run window. If the log is missing, mark the token fields as `pending` and note the missing source in `token_notes`.
