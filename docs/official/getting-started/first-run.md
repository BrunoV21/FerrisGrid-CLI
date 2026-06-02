# First Run

Start with `doctor`, then capture one observation.

```bash
cargo run -q -p ferrisgrid-cli -- doctor
cargo run -q -p ferrisgrid-cli -- observe
```

The observation output includes:

- session directory
- step number
- coordinate mode
- screen IDs
- native and image dimensions
- screenshot paths
- metadata paths

Create one action file:

```bash
cat > .ferrisgrid/action.md <<'EOF'
status: action
action: click
screen_id: screen-1
x: 500
y: 500
button: left
wait_after_ms: 500
EOF
```

Run it:

```bash
cargo run -q -p ferrisgrid-cli -- act --file .ferrisgrid/action.md
```

Use `--dry-run` when you want validation without emitted input.
