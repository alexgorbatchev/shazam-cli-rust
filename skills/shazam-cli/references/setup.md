---
created_on: 2026-10-14 08:50
last_modified: 2026-10-14 08:50
status: current
---

## Credentials and endpoint

Use an existing session: explicit `--cookie` overrides `SHAZAM_COOKIE`; otherwise `--browser` selects one browser and `--profile` selects one profile. Automatic lookup discovers registered browser profiles and returns the first usable profile's Shazam cookie without merging separate profiles. Native cookie filtering enforces request host, path, expiry, secure and isolation context. Windows App-Bound recovery is disabled, so protected cookie values can remain unavailable. Browser reads have native cooperative cancellation and a 30-second budget per operation.

The authenticated endpoint is `https://www.shazam.com/services/profile/v1/en-US/US/web/profile`. Each request uses `limit=100` and the raw-entry offset, including entries omitted from display. Full pages are separated by a cancellable 300 ms pause; each request has a 15-second deadline and an 8 MiB response limit. An explicit empty collection completes pagination. Redirects are returned as failures rather than replaying cookies. Raw error bodies are withheld.

## Cache, exports and streams

The platform cache directory contains `shazam-cli/library.json`: Linux uses `$XDG_CACHE_HOME` or `~/.cache`; other platforms use their native cache directory. `--no-cache` bypasses reads and can still save; `--no-save` suppresses writes. A valid cache is read before credential resolution, including when `--browser` is supplied. Missing/corrupt caches trigger a fetch, with corrupt-cache diagnostics on stderr. Successful empty results replace stale cached tracks. Failed or partial fetches preserve the previous cache.

Only parsed track metadata is saved or exported. Cookies remain in memory. Cache writes and exports use unique same-directory staged files and atomic replacement. Export failures preserve the previous destination. Listing, export acknowledgments, cache state, raw version and skill pointer use stdout; fetch/cache diagnostics and the single returned error use stderr. `AGENT=1`, `true` or `yes` selects compact output. Explicit CSV/query/list output keeps its chosen format. Home paths are folded for display only.

The source project is archived because Apple removed desktop login from Shazam. Fresh authentication cannot be assumed; cached operations remain usable offline. Rust release artifacts and the local usage skill are unpublished.
