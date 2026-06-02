# FerrisGrid Agent Automation Benchmarks

This directory defines the benchmark foundation for comparing FerrisGrid with other computer automation tools when Codex is the executor agent.

The benchmark measures how well Codex can complete the same tasks using each tool's CLI or installed skill instructions. Version 1 is documentation-only: it defines competitors, scenarios, metrics, and run protocol before any fixture apps or harness code are added.

## V1 Scope

Included:

- FerrisGrid CLI plus FerrisGrid skill.
- Playwright CLI plus installed skills.
- Vercel `agent-browser` CLI plus skills.
- Browser Use CLI plus skill.
- Selenium/WebDriver scripts invoked from CLI.
- Browser tasks and desktop tasks.
- Markdown result capture.

Excluded:

- Playwright MCP.
- Chrome DevTools MCP.
- Any other MCP server.
- OpenAI Computer Use / CUA API as a measured competitor.
- Remote autonomous browser agents where Codex is not the executor.
- External websites as benchmark targets.

## Benchmark Rules

- Codex is the executor for every run.
- Each tool gets the same task prompt, success criteria, time budget, and max-step budget.
- Codex may inspect the relevant local skill, CLI help, or official install docs before timed execution.
- Codex must not use MCP tools during benchmark execution.
- Codex must not inspect fixture source during timed execution unless the scenario explicitly allows source inspection.
- Setup time and task execution time are recorded separately.
- Browser-only tools are marked `not_applicable` on desktop-only scenarios instead of failed.
- Every run writes a Markdown result sheet using `results-template.md`.

## Files

- `competitors.md`: v1 tool matrix and inclusion rules.
- `metrics.md`: metric definitions, formulas, and recording guidance.
- `scenarios.md`: initial browser and desktop test cases.
- `runbook.md`: setup, execution, fairness, and repeat protocol.
- `results-template.md`: per-run Markdown capture template.

## Default Environment

Use the Linux Docker workspace described in `../docker-workspace.md` for the first benchmark implementation.

Default display:

```text
XVFB_SCREEN=1280x800x24
```

Default fixture policy:

- Use deterministic local pages and simple local desktop apps.
- Avoid network-dependent pages.
- Avoid authenticated workflows.
- Avoid destructive actions.

