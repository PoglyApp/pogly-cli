# pogly-cli

A Windows and Linux command-line tool for managing a [Pogly](https://pogly.gg) overlay through its HTTP API — and an open-source reference for building your own API integrations.

```
pogly elements add text --text "New follower!" --size 64 --color "#82a5ff" --x 200 --y 150
pogly layouts set-active --name "Starting Soon"
pogly elements list
```

Feedback and questions: https://discord.gg/pogly
API documentation: https://docs.pogly.gg/#http-api

## Install

Quick install (PowerShell):

```powershell
iwr https://cli.pogly.gg -useb | iex
```

This downloads the [latest release](https://github.com/PoglyApp/pogly-cli/releases), installs it to `%LOCALAPPDATA%\Pogly\cli`, and adds it to your `PATH`. The script it runs is [install.ps1](install.ps1) if you'd rather read it first.

Quick install (Bash):

```bash
curl -fsSL https://cli.pogly.gg/install.sh | bash
```

This downloads the [latest release](https://github.com/PoglyApp/pogly-cli/releases), installs it to `~/.local/share/Pogly/cli` (or `$XDG_DATA_HOME/Pogly/cli`), and adds it to your `PATH` via `~/.bashrc` or `~/.zshrc`. The script it runs is [install.sh](install.sh) if you'd rather read it first.

### Manual install

1. Download `pogly` and `pogly-cli` (Linux) or `pogly.exe` and `pogly-cli.exe` (Windows) from the [latest release](https://github.com/PoglyApp/pogly-cli/releases).
2. Place them like this (`<version>` is the release version, e.g. `0.1.0`):

   **Windows:**
   ```
   %LOCALAPPDATA%\Pogly\cli\pogly.exe
   %LOCALAPPDATA%\Pogly\cli\bin\<version>\pogly-cli.exe
   ```

   **Linux:**
   ```
   ~/.local/share/Pogly/cli/pogly
   ~/.local/share/Pogly/cli/bin/<version>/pogly-cli
   ```
   (or under `$XDG_DATA_HOME/Pogly/cli` if that's set)

3. On Linux, `chmod +x` both binaries.
4. Write the version string (no trailing newline) to `version` next to `pogly`/`pogly.exe`.
5. Add that directory to your `PATH`.

### Uninstall

**Windows:** run [uninstall.ps1](uninstall.ps1) — it removes the binaries, version store, and PATH entry. Your overlay profiles are kept unless you pass `-PurgeConfig`.

**Linux:**
```bash
curl -fsSL https://cli.pogly.gg/uninstall.sh | bash
```
Or run [uninstall.sh](uninstall.sh) directly if you already have it locally. It removes the binaries, version store, and PATH entry; overlay profiles are kept unless you pass `--purge-config`.

### Version management

`pogly` (`pogly.exe` on Windows) is a small launcher that runs the selected `pogly-cli` (`pogly-cli.exe` on Windows), so multiple versions can live side by side:

```
pogly version           # show the current version
pogly version upgrade   # install and switch to the latest release
pogly version list      # list installed versions
pogly version use 0.1.0 # switch to another installed version
```

## Quickstart

Mint an API token in Pogly under **Settings → API Access**, then register your overlay:

```
pogly overlay add https://cloud.pogly.gg/overlay?module=<overlay-address> --token pgly_xxxx --nickname main
```

`overlay add` accepts a full overlay URL, a raw overlay address (64 hex chars), or a legacy overlay name. The first overlay you add becomes the default; every other command runs against the default unless you pass `--overlay <nickname>`.

```
pogly whoami
pogly layouts list
pogly elements add image --url "https://cdn.7tv.app/emote/xxxx/4x.webp" --width 128 --height 128
pogly elements update 42 --x 500 --y 300 --locked true
pogly elements delete 42
```

## Commands

| Command | Description |
| --- | --- |
| `ping` | Check that the overlay API is reachable (no token needed) |
| `whoami` | Show the token's label, identity, and permissions |
| `overlay list\|add\|update\|remove\|set-default` | Manage overlay profiles (stored in `%APPDATA%\Pogly\cli\config.toml` on Windows, `~/.config/Pogly/cli/config.toml` on Linux) |
| `elements list\|add <type>\|update\|delete` | Manage elements; `add` has `text`, `image`, `widget`, and `media` subcommands |
| `elementdata list\|add\|update\|delete` | Manage element data assets |
| `layouts list\|add\|duplicate\|rename\|delete\|set-active` | Manage layouts; `set-active` switches the live scene |
| `folders list\|add\|update\|delete` | Manage asset folders |
| `version [list\|use\|upgrade]` | Show, switch, or upgrade the installed CLI version |

Every command supports `--help` for its full flag list, `--json` for the raw API response, and `--overlay <nickname>` to target a specific profile.

Writes require the matching permission on the token owner's account (e.g. `AddElement` for `elements add`); read commands require `Whitelisted`. Read-only tokens are rejected for all writes.

## Examples

The [examples/](examples/) folder shows how to hook the CLI into your stream: Streamer.bot-triggered alerts on subs/raids/channel points, automatic Pogly↔OBS scene sync, live counters, and one rick roll. Anything that can run a program can drive your overlay.

## Building from source

```
cargo build --release
```

On Windows this produces `target\release\pogly-cli.exe` (the CLI) and `target\release\pogly.exe` (the launcher). On Linux it produces `target/release/pogly-cli` and `target/release/pogly`. Run the CLI directly during development: `cargo run -p pogly-cli -- <args>`.

To test `version upgrade` against a fork, set `POGLY_RELEASES_REPO=<owner>/<repo>`.

## License

[MIT](LICENSE)
