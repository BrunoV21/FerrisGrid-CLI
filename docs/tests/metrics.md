# Benchmark Metrics

This document defines the v1 metrics for comparing FerrisGrid with CLI/skill-based automation tools.

## Required Run Fields

Every benchmark run must record:

| Field | Type | Definition |
|---|---|---|
| `run_id` | string | Stable identifier for the run. |
| `scenario_id` | string | Scenario from `scenarios.md`. |
| `tool` | string | Competitor used for the run. |
| `executor` | string | Always `codex` for v1. |
| `result` | enum | `pass`, `partial`, `fail`, or `not_applicable`. |
| `started_at` | timestamp | Wall-clock start of timed execution. |
| `ended_at` | timestamp | Wall-clock end of timed execution. |
| `wall_time_ms` | integer | End-to-end timed task duration. |
| `setup_time_ms` | integer | Install, launch, doctor, browser download, or daemon startup time. |
| `agent_turns` | integer | Codex turns needed to complete or abandon the task. |
| `tool_calls` | integer | CLI/tool invocations made by Codex during timed execution. |
| `actions_total` | integer | User-facing automation actions requested. |
| `clicks_total` | integer | Click actions requested. |
| `failed_clicks` | integer | Clicks that missed, did nothing, hit the wrong target, or needed correction. |
| `recoveries` | integer | Corrective actions after a failed step. |
| `tokens_input` | integer | Input tokens from the Codex session ledger. |
| `tokens_output` | integer | Output tokens from the Codex session ledger. |
| `tokens_total` | integer | `tokens_input + tokens_output`. |
| `trace_quality` | enum | `complete`, `partial`, or `poor`. |
| `notes` | text | Short run-specific observations. |

## Core Formulas

```text
wall_time_ms = ended_at - started_at
tokens_total = tokens_input + tokens_output
failure_per_click = failed_clicks / clicks_total
actions_per_success = actions_total / successful_task_count
recoveries_per_run = recoveries
```

If `clicks_total` is `0`, record `failure_per_click: n/a`.

If `result` is not `pass`, do not compute `actions_per_success` for that run.

## Result Semantics

### `pass`

The task reaches the exact success state defined by the scenario.

### `partial`

The run makes meaningful progress but misses at least one required assertion or needs human judgment to confirm completion.

### `fail`

The run does not reach the required success state, times out, exceeds max steps, crashes, or cannot recover from an incorrect action.

### `not_applicable`

The scenario is outside the tool's supported surface. Example: Selenium on a desktop-only terminal task.

## Action Counting

Count these as actions:

- Clicks.
- Typing text.
- Keypresses and hotkeys.
- Scrolls.
- Browser navigation commands.
- Script executions that directly interact with the target app.
- Screenshot or observe commands only when they are part of the tool's normal interaction loop.

Do not count:

- Install commands.
- Doctor/check commands.
- File reads for documentation.
- Result logging commands.

## Failed Click Counting

Count a failed click when any of these occur:

- The click visibly misses the intended element.
- The click triggers the wrong UI state.
- The click produces no change where the expected behavior requires change.
- Codex must click again because the first target was wrong.
- The tool reports an action failure for the click.

Do not count a failed click when:

- The click is intentionally exploratory and the scenario allows exploration.
- A UI legitimately requires two clicks and Codex planned both.
- A click succeeds but a later assertion fails for another reason.

## Token Accounting

Use the Codex session ledger as the v1 source of truth:

- Record input, output, and total tokens for the timed task segment.
- Exclude setup and documentation-inspection tokens when possible.
- If the ledger cannot isolate setup from timed execution, record total session tokens and add a note.

When exact token counts are unavailable, mark token fields `unknown` and include transcript size estimates in `notes`. Do not mix estimated tokens with exact ledger tokens in aggregate charts.

## Trace Quality

### `complete`

The run has enough artifacts to reconstruct what happened:

- Tool commands.
- Tool outputs.
- Screenshots or snapshots where relevant.
- Action history.
- Final assertion evidence.

### `partial`

The run has enough artifacts to understand the final state but not every intermediate action.

### `poor`

The run cannot be debugged without rerunning it.

## Aggregate Reporting

For each tool and scenario, run at least 3 trials and report:

- Pass rate.
- Median wall time.
- p95 wall time when at least 10 trials exist.
- Median tokens total.
- Median tool calls.
- Mean failure per click.
- Mean recoveries.
- Trace quality distribution.

