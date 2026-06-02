# Benchmark Runbook

This runbook defines how to execute v1 FerrisGrid comparison benchmarks.

## 1. Prepare Environment

Use the FerrisGrid Linux workspace:

```bash
docker build -f docker/linux-workspace.Dockerfile -t ferrisgrid-linux-workspace .
docker run --rm -d \
  --name ferrisgrid-workspace \
  -p 6080:6080 \
  -e XVFB_SCREEN=1280x800x24 \
  -v "$PWD:/workspace" \
  ferrisgrid-linux-workspace
```

Verify FerrisGrid:

```bash
docker exec ferrisgrid-workspace ferrisgrid doctor
docker exec ferrisgrid-workspace ferrisgrid observe --max-image-edge 1280
```

For browser competitors, install each CLI in the host or container according to the competitor-specific setup chosen for the benchmark implementation. Record setup time separately from task execution time.

## 2. Prepare Tool Instructions

Before timed execution, Codex may inspect:

- The installed tool skill.
- Local CLI help.
- Official install docs.
- The benchmark scenario prompt and success criteria.

Codex must not inspect fixture implementation source during timed execution unless the scenario explicitly allows it.

## 3. Start A Timed Run

For each tool/scenario/trial:

1. Reset the fixture to its initial state.
2. Reset or isolate browser/session state for the tool.
3. Start wall-clock timing.
4. Give Codex the scenario prompt and allowed tool instructions.
5. Let Codex execute using only the selected competitor surface.
6. Stop timing when the success state is reached, failure is declared, max turns are reached, or timeout occurs.
7. Fill out `results-template.md`.

## 4. Tool-Specific Rules

### FerrisGrid

- Use `ferrisgrid observe` and `ferrisgrid act`.
- Use the FerrisGrid skill instructions.
- Alternate observation and one action at a time.
- Record each observe and act as a tool call.
- Count each requested `click`, `type`, `press_key`, `scroll`, `drag`, or `move_mouse` as an action.

### Playwright CLI

- Use Playwright CLI and installed skills only.
- Do not use Playwright MCP.
- Browser task execution may use snapshots, refs, screenshots, generated scripts, or CLI-supported interactions.
- Count generated script executions that interact with the page as actions when they directly perform browser interactions.

### Vercel `agent-browser`

- Use `agent-browser` CLI and bundled skills.
- Do not replace Codex with an autonomous external agent loop.
- Count `open`, `snapshot`, `click`, `fill`, `type`, `screenshot`, and similar commands as tool calls.
- Count page interactions as actions.

### Browser Use CLI

- Use `browser-use` CLI and skill instructions.
- Local browser mode is the default.
- Cloud-backed browser mode may be tested only as a labeled variant.
- Browser Use cloud task APIs that run a separate autonomous agent are excluded.

### Selenium/WebDriver

- Use Selenium scripts invoked from CLI.
- Count Codex script-authoring turns as part of timed execution unless a scenario explicitly measures prewritten scripts.
- Count script execution as one tool call.
- Count browser interactions inside the script as actions where they can be inferred from the script or logs.

## 5. Fairness Rules

- Same task prompt for every tool.
- Same fixture reset for every trial.
- Same max turns and timeout for every tool.
- Same browser viewport and display size where possible.
- No MCP tools.
- No external websites.
- No authenticated sessions.
- No manual human correction during timed execution.
- Human may stop unsafe behavior and mark the run failed with notes.

## 6. Repetition And Aggregation

Minimum:

- 3 trials per tool/scenario pair.

Preferred after fixture stabilization:

- 10 trials per tool/scenario pair.

Aggregate by tool and scenario:

- Pass rate.
- Median wall time.
- Median tokens total.
- Median tool calls.
- Mean failure per click.
- Mean recoveries.
- Trace quality distribution.

## 7. Result Storage

Store run outputs under a future benchmark results directory when implementation begins. Until then, each run may be captured as a standalone Markdown file copied from `results-template.md`.

Suggested future layout:

```text
docs/tests/results/
  2026-06-cli-skill-v1/
    browser-button-state/
      ferrisgrid-run-001.md
      playwright-cli-run-001.md
      agent-browser-run-001.md
```

