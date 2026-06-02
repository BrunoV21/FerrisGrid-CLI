# Benchmark Competitors

This document defines the v1 competitor set for FerrisGrid benchmarks.

## Inclusion Rule

A v1 competitor must be usable by Codex through a CLI, a local script invoked from a CLI, or an installed skill. MCP-only and API-only competitors are excluded from v1.

## V1 Competitor Matrix

| Tool | Mode | Primary Surface | Task Coverage | Included |
|---|---|---|---|---|
| FerrisGrid | Visual desktop control | `ferrisgrid observe`, `ferrisgrid act`, FerrisGrid skill | Browser and desktop | Yes |
| Playwright CLI | Browser automation for agents | `playwright-cli`, installed skills | Browser | Yes |
| Vercel `agent-browser` | Browser automation CLI for agents | `agent-browser`, bundled skills | Browser | Yes |
| Browser Use CLI | Persistent browser automation CLI | `browser-use`, optional skill | Browser | Yes |
| Selenium/WebDriver | Scripted browser automation | Python or JavaScript scripts run from CLI | Browser | Yes |

## Explicitly Excluded From V1

| Tool | Reason |
|---|---|
| Playwright MCP | MCP baseline, not CLI/skill-only. |
| Chrome DevTools MCP | MCP baseline, not CLI/skill-only. |
| OpenAI Computer Use / CUA API | API/model loop where Codex is not the only executor agent. |
| Browser Use cloud agent tasks | Remote autonomous agent loop instead of Codex driving the CLI directly. |
| Hosted RPA tools | Outside current local CLI/skill priority. |

## Competitor Profiles

### FerrisGrid

FerrisGrid is the target tool. It exposes single-step visual computer control through compact Markdown:

- `ferrisgrid observe` captures screen state and writes screenshots/metadata.
- `ferrisgrid act` executes one constrained action and captures the result.
- Codex alternates observation, reasoning, and action through the FerrisGrid skill.

FerrisGrid is eligible for both browser and desktop scenarios.

### Playwright CLI

Playwright CLI is included as a browser automation CLI designed for coding-agent workflows. Codex should use installed skills or local CLI help for current command details.

V1 uses Playwright CLI only. Playwright MCP is excluded.

### Vercel `agent-browser`

`agent-browser` is included as a purpose-built browser automation CLI for AI agents. It can expose page snapshots, refs, clicks, fills, screenshots, and bundled skills.

V1 uses local CLI/skill flows only.

### Browser Use CLI

Browser Use CLI is included for persistent browser automation from the command line. Codex should drive it through commands such as opening pages, reading state, clicking elements, typing, taking screenshots, and running CLI-supported scripts.

Cloud browser control is allowed only when Codex is still directly driving the CLI and the runbook explicitly labels the run as cloud-backed. Remote autonomous task execution is excluded.

### Selenium/WebDriver

Selenium is included as a conventional script baseline. Codex may write or run small browser automation scripts from the CLI, using the same fixture and success criteria.

Selenium is not agent-native, so metrics should distinguish:

- Codex script-authoring time.
- Script execution time.
- Failures caused by generated locator/script errors.

## Sources To Recheck Before Implementation

These product surfaces can change. Before building fixture or harness code, recheck official docs or repositories for install and command details:

- FerrisGrid local docs in this repository.
- Playwright CLI official repository/docs.
- Vercel `agent-browser` repository.
- Browser Use CLI docs.
- Selenium WebDriver docs.

