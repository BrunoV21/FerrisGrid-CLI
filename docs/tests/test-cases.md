# Host Benchmark Test Cases

These are the concrete operations the tester gives Codex for each scenario. The same prompt and success criteria must be used for every competitor.

## Browser Fixtures

All browser cases use:

```text
http://127.0.0.1:4173/?scenario=<scenario_id>&reset=1
```

### `browser-button-state`

Task prompt:

```text
Open the local fixture. Click the Activate Reactor button. Finish only when the page visibly says "Status: Flux stabilized".
```

Operations under test:

- Open a local URL.
- Locate a labeled button.
- Click the correct button.
- Verify changed visible text.

Success criteria:

- The page opens at the button-state scenario.
- The Activate Reactor button is clicked.
- The final visible status is `Status: Flux stabilized`.

### `browser-form-validation`

Task prompt:

```text
Open the local fixture. Fill the contact form with name "Ada Lovelace", email "ada@example.test", and message "Benchmark message accepted.". Submit the form. Finish only when the page visibly says "Submission received for Ada Lovelace".
```

Operations under test:

- Locate text fields and a textarea by visible labels.
- Type exact values.
- Submit the form.
- Verify the final success message.

Success criteria:

- Name, email, and message fields contain the requested values before submit.
- Submit action succeeds.
- The final visible result is `Submission received for Ada Lovelace`.

### `browser-multi-step-wizard`

Task prompt:

```text
Open the local fixture. Complete the wizard by choosing Stable Route, entering operator "Mina Patel" and code "Q4-17", then confirming. Finish only when the final summary says "Stable Route | Mina Patel | Q4-17".
```

Operations under test:

- Choose the correct option despite nearby ambiguous labels.
- Advance through a multi-step flow.
- Fill exact text values.
- Confirm the summary.

Success criteria:

- Step 1 selection is `Stable Route`.
- Step 2 values are operator `Mina Patel` and code `Q4-17`.
- Step 3 is confirmed.
- Final summary is `Stable Route | Mina Patel | Q4-17`.

### `browser-table-filter`

Task prompt:

```text
Open the local fixture. Filter the inventory table for "cobalt". Finish only when the only visible data row is "Cobalt Ridge".
```

Operations under test:

- Locate a filter/search input.
- Type an exact filter term.
- Inspect table row visibility.
- Verify only the expected row remains visible.

Success criteria:

- Filter term is `cobalt`.
- `Cobalt Ridge` remains visible.
- Non-matching inventory rows are hidden.

### `browser-scroll-target`

Task prompt:

```text
Open the local fixture. Scroll down to Archive Node 42 and select it. Finish only when the page visibly says "Selected Archive Node 42".
```

Operations under test:

- Scroll through a long local page.
- Locate an offscreen target.
- Click the target action.
- Verify the final selected state.

Success criteria:

- The target item `Archive Node 42` is reached.
- Its Select button is clicked.
- The final visible state is `Selected Archive Node 42`.

### `desktop-coordinate-stability`

Task prompt:

```text
Open the local fixture in a visible browser window. Click the Click Target button five times using the visual UI only. Finish only when the counter says "Successful clicks: 5".
```

Operations under test:

- Use visual target selection repeatedly.
- Keep coordinates stable across observation/action cycles.
- Verify the repeated-click counter.

Success criteria:

- The same target receives five successful clicks.
- No coordinate drift causes a missed click.
- The final visible state is `Successful clicks: 5`.

## Desktop Host Cases

Desktop host cases are executed with the local host UI and FerrisGrid visual control. Browser-only competitors are recorded as `not_applicable`.

### `desktop-terminal-command`

Task prompt:

```text
Open a host terminal window. Type and run: printf 'FG_TERMINAL_OK\n'. Finish only when the visible terminal output contains "FG_TERMINAL_OK".
```

Operations under test:

- Focus the correct host terminal window.
- Type a command exactly.
- Press Enter.
- Verify visible terminal output.

Success criteria:

- A terminal window is visible.
- The command is entered and submitted.
- Visible output contains `FG_TERMINAL_OK`.

### `desktop-window-switch`

Task prompt:

```text
Open two visible browser windows with the local fixture: one on browser-button-state and one on browser-table-filter. Switch to the table-filter window, filter for "cobalt", and leave the button-state window unchanged.
```

Operations under test:

- Distinguish two visible windows.
- Focus the requested target window.
- Act only in the target window.
- Verify the non-target window did not change.

Success criteria:

- Two target windows are visible.
- The table-filter window is selected.
- The table-filter window shows only `Cobalt Ridge`.
- The button-state window still shows `Status: waiting`.
