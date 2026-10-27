---
name: shazam-cli
description: Use when listing, refreshing, exporting or managing a Shazam history library with shazam-cli.
author: alexgorbatchev
metadata:
  created_on: 2026-10-14 08:50
  last_modified: 2026-10-14 08:50
  status: current
---

Run `AGENT=1 shazam-cli skill` for the canonical installation pointer. Read [commands](references/commands.md) for all commands, arguments and output contracts; read [setup](references/setup.md) for credentials, browser lookup and cache behavior.

Use the cached library before fetching fresh data. For query pipelines, run:

```sh
AGENT=1 shazam-cli library list --output query --suffix "Extended Mix"
```

For an explicit refresh, supply an existing session through `SHAZAM_COOKIE` and run `AGENT=1 shazam-cli library list --no-cache`. For an offline export, run `AGENT=1 shazam-cli library export library.json`.

Treat exit 0 as success, 1 as a runtime failure, 2 as invalid arguments and 130 as cancellation. Keep stdout data and stderr diagnostics distinct. Cache status reports `empty`, `corrupt` or `ready`; a corrupt status is an observation with exit 0, while filesystem access errors fail.

The source tool is archived because desktop Shazam login was removed. Use an already usable session or the cache. Rust artifacts and this local Rust skill are unpublished; the printed GitHub pointer names the canonical source location with publication pending.
