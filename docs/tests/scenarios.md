# Benchmark Scenarios

This document defines the v1 benchmark scenario catalog.

Each scenario uses deterministic local fixtures. External websites are excluded from v1 to avoid network, auth, layout, and anti-bot variability.

## Shared Defaults

- Executor: Codex.
- Display: `1280x800`.
- Max timed task duration: 10 minutes.
- Max agent turns: 30.
- Minimum trials per tool: 3.
- Success evidence: final visible state, command output, screenshot, snapshot, or assertion log.

## Browser Scenarios

### `browser-button-state`

Goal: open a local page, click a labeled button, and verify that visible text changes.

Success criteria:

- The page opens successfully.
- The target button is clicked.
- The final page contains the expected changed text.

Primary metrics:

- Time to first correct click.
- Failed clicks.
- Tool calls.
- Tokens total.

### `browser-form-validation`

Goal: fill a local form with name, email, and message fields, submit it, and verify the success state.

Success criteria:

- Required fields contain the requested values.
- Submit action succeeds.
- Final page shows the expected success message.

Primary metrics:

- Actions total.
- Recoveries.
- Failed clicks.
- Tokens total.

### `browser-multi-step-wizard`

Goal: complete a three-step local wizard with one intentionally ambiguous label.

Success criteria:

- Step 1 selection is correct.
- Step 2 form values are correct.
- Step 3 confirmation is submitted.
- Final summary matches expected values.

Primary metrics:

- Recoveries.
- Failed clicks.
- Agent turns.
- Trace quality.

### `browser-table-filter`

Goal: use search or filter controls in a table and verify one expected row remains visible.

Success criteria:

- The correct filter term is entered.
- The expected row is visible.
- Non-matching rows are hidden or clearly not selected.

Primary metrics:

- Tool calls.
- Tokens total.
- Wall time.

### `browser-scroll-target`

Goal: scroll to offscreen content and interact with a target near the bottom of a local page.

Success criteria:

- The target item is reached.
- The target action is performed.
- Final state confirms the correct item was selected.

Primary metrics:

- Scroll actions.
- Failed clicks.
- Recoveries.
- Wall time.

## Desktop Scenarios

Desktop scenarios are mandatory for FerrisGrid and `not_applicable` for browser-only tools unless the tool can directly control the desktop through a CLI/skill surface.

### `desktop-chromium-visible-ui`

Goal: use Chromium through visible desktop UI, not DOM selectors, to navigate to a local fixture and perform a simple action.

Success criteria:

- Chromium is visible in the Linux workspace.
- The local fixture opens.
- The requested UI action succeeds.
- Final visual state matches expected text.

Primary metrics:

- Observe/action loop count.
- Failed clicks.
- Coordinate accuracy notes.
- Wall time.

### `desktop-terminal-command`

Goal: use `xterm` to type and run a command, then verify visible terminal output.

Success criteria:

- `xterm` is visible.
- The command is typed and submitted.
- Expected output is visible.

Primary metrics:

- Failed clicks.
- Typing/key actions.
- Recoveries.
- Trace quality.

### `desktop-window-switch`

Goal: switch between two visible windows and perform the target action in the correct window.

Success criteria:

- Two target windows are visible.
- Codex selects the requested window.
- The action affects the correct window only.

Primary metrics:

- Wrong-window actions.
- Failed clicks.
- Recoveries.
- Agent turns.

### `desktop-coordinate-stability`

Goal: validate that repeated clicks on a fixed visual target remain accurate across observe/act cycles.

Success criteria:

- The same target is clicked successfully across repeated cycles.
- No coordinate drift causes a miss.
- Final state records the expected number of successful clicks.

Primary metrics:

- Failure per click.
- Coordinate drift notes.
- Observe/action latency.
- Trace quality.

## Scenario Authoring Rules

When implementation begins, each fixture should provide:

- A human-readable task prompt.
- A deterministic initial state reset.
- A clear pass/fail assertion.
- A local artifact showing final state.
- No hidden dependencies on internet access.
- No login, payment, email, or destructive workflow.

