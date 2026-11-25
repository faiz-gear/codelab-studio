# Workspace browser smoke test

Install the Chromium headless shell once after installing the project's dependencies, then run the test from the repo root.

```sh
pnpm exec playwright install chromium --only-shell
pnpm test:browser
```

Playwright starts Vite on port 1421 and mounts the workbench with `FakeCoreClient`. The test uses the real Monaco editor and simulates external writes through the fake. Failure traces are saved under `test-results/`.

With Google Chrome already installed, set `CODELAB_BROWSER_CHANNEL=chrome` when running `pnpm test:browser` to use that installation.
