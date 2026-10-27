`shazam-cli` lists and exports a Shazam history library from a saved cache or an existing web session. It serves users who already have usable session cookies; the archived source project reports that Shazam removed desktop login, so acquiring a new session through a desktop browser is not generally available.

# What It Does

- **Read history:** Fetch all available library pages using an existing session.
- **Keep an offline copy:** Save track metadata and list it without contacting Shazam.
- **Export tracks:** Write JSON or CSV, or print search queries with a chosen suffix.
- **Manage the cache:** Inspect its state or remove it explicitly.

# How It Works

- Run `shazam-cli` or `shazam-cli library list` to read the saved library first.
- Add `--no-cache` to fetch fresh pages. Supply `--cookie`, set `SHAZAM_COOKIE`, or select an existing browser profile with `--browser` and `--profile`.
- Choose `--output query`, `list`, `csv`, or `table`; use `library export FILE` for a persistent export.

# How it Really Works

- Fresh requests send the session cookie to Shazam's private profile endpoint. They fetch 100 entries per page, pause 300 ms between full pages and use a 15-second request deadline. Responses are limited to 8 MiB per page.
- Browser lookup reads one profile at a time and filters cookies for the exact request context. Cookies remain in memory. Windows App-Bound recovery is disabled; protected sessions can be unavailable.
- Track metadata is saved atomically at the platform cache directory under `shazam-cli/library.json`. A successful empty library replaces stale entries. `--no-cache` bypasses reads; `--no-save` suppresses writes.
- JSON export retains track keys, titles, artists, optional UTC timestamps, URLs and optional cover-art URLs. CSV exports include `Index,Date,Title,Artist,URL,CoverArt,Key`. An export replaces its destination atomically.
- Lists and exports go to stdout; fetch diagnostics go to stderr. `AGENT=1` makes table output compact key-value records. Explicit CSV, query and list formats retain their chosen shape.
- Exit status is 0 for success, 1 for runtime failures, 2 for invalid arguments and 130 for cancellation. Authentication failure preserves the existing cache. Error bodies and cookies are withheld.

# Installation

Rust release artifacts are unpublished. Existing source-project releases are not Rust distributions.

# Setup

- A usable existing [Shazam](https://www.shazam.com/) session is required for a fresh fetch. `SHAZAM_COOKIE` or `--cookie` supplies it directly; cached operations need no session.
- Browser extraction depends on the platform's available cookie stores and decryption facilities. Desktop login availability remains an upstream limitation.

# Quick Start

```sh
shazam-cli cache status
shazam-cli library list --output query --suffix "Extended Mix"
shazam-cli library export library.json
AGENT=1 shazam-cli library list
```

Sample output for `--output query`:

```text
Artist - Song Extended Mix
```

# Options & Flags

All flags are global.

| Flag | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--cookie <COOKIE>` | `-c` | `SHAZAM_COOKIE`, then browser lookup | Existing session cookie, kept in memory |
| `--browser <BROWSER>` | `-b` | Automatic lookup | Native browser identifier |
| `--profile <PROFILE>` | `-p` | `Default` | Selected browser's profile selector |
| `--output <OUTPUT>` | `-o` | `table` | `table`, `list`, `csv`, or `query` |
| `--suffix <SUFFIX>` | | Empty | Text appended to query lines |
| `--no-cache` | | `false` | Bypass saved library reads |
| `--no-save` | | `false` | Suppress saving fetched metadata |
| `--version` | | `false` | Print raw version and exit |
| `--help` | `-h` | `false` | Print command help |

The offline `shazam-cli skill` command prints the canonical source skill installation pointer. Its Rust usage file exists locally; publication at that URL is pending.

# License

[MIT](LICENSE), copyright 2026 Alex Gorbatchev.
