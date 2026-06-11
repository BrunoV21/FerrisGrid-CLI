# Browser Benchmark Fixture

This directory contains a deterministic local browser fixture for `docs/tests/test-cases.md`.

Run it from the repository root:

```bash
python3 -m http.server 4173 --directory docs/tests/fixtures/browser
```

Open a scenario:

```text
http://127.0.0.1:4173/?scenario=browser-button-state&reset=1
```

Validate the fixture:

```bash
npx -y -p playwright node docs/tests/fixtures/browser/playwright-smoke.mjs http://127.0.0.1:4173/
```
