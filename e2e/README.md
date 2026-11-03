# BlueOS e2e

Playwright suite that walks every BlueOS page on a real vehicle, read-only. Each test records a `recording.webm`
with a title card and a visible cursor, attached to the HTML report.

```bash
bun --cwd e2e install
bun --cwd e2e run e2e:install-browser
bun --cwd e2e run e2e                      # full suite
bun --cwd e2e run e2e:changed              # only the pages touched by the diff against origin/master
bun --cwd e2e run e2e:failed               # re-run the last failures
bun --cwd e2e run e2e:report
```

The vehicle address comes from `BLUEOS_URL`, default `http://192.168.2.2`:

```bash
BLUEOS_URL=http://blueos.local bun --cwd e2e run e2e
```

## What the suite touches

The global setup enables pirate mode through the tray menu and the global teardown disables it again. The fixture
answers the setup wizard check locally and skips the guided tour if it shows up, so nothing else is written to the
vehicle. Anything that reboots, changes network, flashes firmware, writes parameters, installs or removes extensions,
updates BlueOS, changes video settings, arms the vehicle, sends MAVLink, deletes files or runs commands is only
asserted present, marked with `// DESTRUCTIVE – not exercised` in the spec.
The speed tests on the Network Test page are also left alone: they are harmless but slow.

## Running only what changed

`impact-map.json` maps frontend source paths to the page tags. `select-tests.mjs` diffs the working tree against a
ref (default `origin/master`, or `--staged`), applies the map and prints a `--grep`, or runs it with `--run`. A
changed spec file selects its own tag. `check-impact-map.mjs` fails when a tag in a spec or in the map is missing from
the other, and runs first in `e2e:changed`.

The map is conservative on purpose: a frontend file that matches no rule runs the full suite, and the script prints
`unmatched: <file> → running full suite` so the map gets extended. Never fix an unmatched file with a rule that
selects nothing. Non-frontend changes select nothing.

CI: pull requests run `e2e:changed`, the master and nightly jobs run the full suite so the map cannot rot silently.
