# Agent Protocol

FerrisGrid speaks compact Markdown at the process boundary.

## Observation

An agent calls:

```bash
ferrisgrid observe
```

FerrisGrid returns screenshot paths, screen IDs, dimensions, coordinate mode, and metadata paths. The agent inspects the screenshot and chooses one next action.

## Action

The agent sends one constrained action block:

```yaml
status: action
action: click
screen_id: screen-1
x: 500
y: 500
button: left
wait_after_ms: 500
```

FerrisGrid validates and executes at most one action, then captures the post-action screen state.

## Done and fail

Agents can also report terminal states:

```yaml
status: done
reason: task complete
```

```yaml
status: fail
reason: target control is not visible
```

These states are part of the same bounded protocol: the agent decides, FerrisGrid records and reports.
