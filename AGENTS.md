---
created_on: 2026-10-14 08:50
last_modified: 2026-10-14 08:50
status: current
---

# Shazam history CLI development

Inherit the parent Rust instructions. This is an independent Cargo package and Git repository.

## Development

- Build: `just build` writes `bin/shazam-cli`.
- Run: `just run <args>` or `just run-ai <args>`.
- Test: `just test`; format this package only: `just fmt`.
- Gate changes: `just check` runs formatting, strict Clippy and at least 90% line coverage.
- Any runtime behavior change requires a corresponding test-file change and 90% line coverage; `scripts/` is excluded.
- Keep evidence in `.tmp/`. Use synthetic SQLite cookie databases and local HTTP servers in tests, never real browser profiles or sessions.

## Contracts

- Keep the root default library listing. Group explicit library operations under `library` and cache operations under `cache`.
- Keep `skills/shazam-cli/` synchronized with commands, flags, defaults, environment, streams, exit statuses and side effects. Verify it against the live Clap tree.
- Cookie precedence is explicit flag, `SHAZAM_COOKIE`, selected browser, then automatic native browser discovery. Cache hits precede credential lookup unless `--no-cache` is supplied.
- Browser reads select one profile at a time; never merge unrelated session cookies. Use the native request-header context to enforce domain, path, expiry, secure and partition boundaries.
- Keep browser App-Bound recovery disabled in both Cargo features and request execution policy. Never log, persist or export cookies. Cache only parsed track metadata.
- The original project is archived because desktop Shazam login was removed. Preserve that limitation and never imply successful authentication or API support from local tests.
- Treat an explicit empty collection as a completed page. Advance offsets by raw collection length, including entries omitted from display. Preserve earlier cache bytes on any fetch failure.
- Bound HTTP responses and retain cancellation during requests, bodies and pagination delays. Never replay requests through redirects with session credentials.
- Export and cache replacement use unique same-directory temporary files and atomic persistence. Propagate output failures.
- Returned errors are printed exactly once by `main`; keep stdout machine data distinct from diagnostics. Fold displayed home paths.
- Rust release artifacts and the canonical source skill URL are unpublished. Never invent download URLs or publish releases, tags, packages or production deployments without explicit user authorization.
