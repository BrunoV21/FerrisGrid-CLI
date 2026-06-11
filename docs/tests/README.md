# FerrisGrid Agent Automation Benchmarks

This directory defines the benchmark foundation for comparing FerrisGrid with other computer automation tools when Codex is the executor agent.

The benchmark measures how well Codex can complete the same tasks using each tool's CLI or installed skill instructions. Version 1 starts with host-only local fixtures so the same task prompts can be executed by FerrisGrid and browser automation competitors without depending on external websites.

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
- `host-runbook.md`: host-only execution protocol for local benchmark runs.
- `test-cases.md`: exact task prompts, setup, and success criteria used by the executor.
- `results-template.md`: per-run Markdown capture template.
- `fixtures/`: deterministic local fixtures and smoke runners.

## Default Environment

Use the host machine for the current benchmark implementation. Do not use Docker for host benchmark runs.

Default host display target:

```text
1280x800 or the closest available visible browser/desktop window size
```

Default fixture policy:

- Use deterministic local pages and simple local desktop apps.
- Avoid network-dependent pages.
- Avoid authenticated workflows.
- Avoid destructive actions.
