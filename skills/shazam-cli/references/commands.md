---
created_on: 2026-10-14 08:50
last_modified: 2026-10-14 08:50
status: current
---

## `shazam-cli`

Root invocation lists library tracks. No positional arguments. All flags below are global and accepted on every command.

| Flag | Short | Type | Default / accepted values |
| --- | --- | --- | --- |
| `--cookie` | `-c` | string | Explicit value, then `SHAZAM_COOKIE`, otherwise browser lookup |
| `--browser` | `-b` | string | Automatic lookup; native registered browser identifier such as `chrome`, `vivaldi`, `firefox` or platform-supported `safari` |
| `--profile` | `-p` | string | `Default`; profile ID, display name, directory name or full path for a selected browser |
| `--output` | `-o` | value (enum) | `table`; `table`, `list`, `csv`, `query` |
| `--suffix` | | string | Empty; appended to each query with one separating space |
| `--no-cache` | | bool | `false`; fetch instead of reading cached tracks |
| `--no-save` | | bool | `false`; keep fetched tracks out of the cache |
| `--version` | | bool | `false`; print strictly the raw version plus newline |
| `--help` | `-h` | bool | `false`; print help and exit |

## `shazam-cli library`

Require a library subcommand. `--help` renders its complete subtree.

## `shazam-cli library list`

List tracks using the selected format. `query` prints `Artist - Title` plus the optional suffix. `list` prints `[INDEX] Title - Artist (DATE)`. `csv` prints `Index,Date,Title,Artist,URL` with proper quoting. Human `table` renders a table; agent `table` prints `index`, `date`, `title`, `artist`, `url` key-value records. Missing dates are `N/A`. An empty library emits no track rows.

## `shazam-cli library export <FILE>`

Require one destination path. A case-insensitive `.json` extension selects a pretty JSON track array; all other extensions select CSV with `Index,Date,Title,Artist,URL,CoverArt,Key`. JSON fields are `key`, `title`, `artist`, optional UTC `timestamp`, `url`, optional `cover_art`. Create destination parents and atomically replace the destination. Print `Exported COUNT tracks to: PATH` on stdout, folding the home prefix to `~` for display.

## `shazam-cli cache`

Require a cache subcommand.

## `shazam-cli cache status`

Read local metadata only. Print `cache: PATH` then `status: empty`, `status: corrupt` with `bytes`, or `status: ready` with `tracks`, `bytes`, `modified` UTC timestamp. Corrupt JSON reports a successful observation; inaccessible files fail.

## `shazam-cli cache clear`

Remove the saved library file and print `cache: PATH` and `status: cleared`. Already absent is successful. No authentication or network request.

## `shazam-cli skill`

Accept no positional arguments or command-specific flags. Print the Skills CLI installation command and canonical source GitHub skill URL with identical bytes in human and agent modes. Perform no network request or repository-file read.

## `shazam-cli help [COMMAND]...`

Generated Clap help command. Accept a command path and render its help. Every agent-mode help path starts with the alert directing agents to `AGENT=1 shazam-cli skill`.

Supported generated paths: `shazam-cli help cache`, `shazam-cli help cache clear`, `shazam-cli help cache status`, `shazam-cli help library`, `shazam-cli help library export`, `shazam-cli help library list`, `shazam-cli help skill`, `shazam-cli help help`; `shazam-cli cache help`, `shazam-cli cache help clear`, `shazam-cli cache help status`, `shazam-cli cache help help`; `shazam-cli library help`, `shazam-cli library help export`, `shazam-cli library help list`, `shazam-cli library help help`.
