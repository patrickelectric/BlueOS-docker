# BlueOS e2e

Playwright suite with one file per BlueOS page. Each file tests a minimal feature of its page on a real vehicle, with
the frontend running in pirate mode.

```bash
bun install --cwd e2e
bunx --cwd e2e playwright install chromium
BLUEOS_URL=http://blueos.local bun --cwd e2e e2e
```

The vehicle address comes from `BLUEOS_URL`, default `http://192.168.2.2`.

The version chooser switches to a core image made with `docker save` when `BLUEOS_CORE_IMAGE` points to its `.tar`,
the image frontend needs the version chooser test ids. The vehicle goes back to the version it was running.

Pirate mode and the language are frontend settings, the fixture answers the settings request with pirate mode on and
English, so nothing is written on the vehicle.
