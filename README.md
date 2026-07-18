# pogly-cli

A Windows command-line tool for managing a [Pogly](https://pogly.gg) overlay through its HTTP API — and an open-source reference for building your own API integrations.

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

### Manual install

1. Download `pogly.exe` and `pogly-cli.exe` from the [latest release](https://github.com/PoglyApp/pogly-cli/releases).
2. Place them like this (`<version>` is the release version, e.g. `0.1.0`):
   ```
   %LOCALAPPDATA%\Pogly\cli\pogly.exe
   %LOCALAPPDATA%\Pogly\cli\bin\<version>\pogly-cli.exe
   ```
3. Add `%LOCALAPPDATA%\Pogly\cli` to your `PATH`.

### Uninstall

Run [uninstall.ps1](uninstall.ps1) — it removes the binaries, version store, and PATH entry. Your overlay profiles are kept unless you pass `-PurgeConfig`.

### Version management

`pogly.exe` is a small launcher that runs the selected `pogly-cli.exe`, so multiple versions can live side by side:

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
| `overlay list\|add\|update\|remove\|set-default` | Manage overlay profiles (stored in `%APPDATA%\Pogly\cli\config.toml`) |
| `elements list\|add <type>\|update\|delete` | Manage elements; `add` has `text`, `image`, `widget`, and `media` subcommands |
| `elementdata list\|add\|update\|delete` | Manage element data assets |
| `layouts list\|add\|duplicate\|rename\|delete\|set-active` | Manage layouts; `set-active` switches the live scene |
| `folders list\|add\|update\|delete` | Manage asset folders |
| `osc` | Run the OSC listener server to control the overlay via UDP |
| `version [list\|use\|upgrade]` | Show, switch, or upgrade the installed CLI version |

Every command supports `--help` for its full flag list, `--json` for the raw API response, and `--overlay <nickname>` to target a specific profile.

Writes require the matching permission on the token owner's account (e.g. `AddElement` for `elements add`); read commands require `Whitelisted`. Read-only tokens are rejected for all writes.

## Examples

The [examples/](examples/) folder shows how to hook the CLI into your stream: Streamer.bot-triggered alerts on subs/raids/channel points, automatic Pogly↔OBS scene sync, live counters, and one rick roll. Anything that can run a program can drive your overlay.

## Model Context Protocol (MCP) Server

`pogly-cli` can run as a Model Context Protocol (MCP) server, allowing AI assistants (like Claude Desktop, Cursor, or Windsurf) to view, build, and manage your Pogly overlay layouts and elements directly.

To start the MCP server using `stdio` transport:

```
pogly mcp
```

### Claude Desktop Integration

Add this to your `claude_desktop_config.json` (typically at `~/Library/Application Support/Claude/claude_desktop_config.json` on macOS, or `%APPDATA%\Claude\claude_desktop_config.json` on Windows):

```json
{
  "mcpServers": {
    "pogly": {
      "command": "pogly",
      "args": ["mcp"]
    }
  }
}
```

Or for development / running from source:

```json
{
  "mcpServers": {
    "pogly": {
      "command": "cargo",
      "args": [
        "run",
        "--manifest-path",
        "/absolute/path/to/pogly-cli/Cargo.toml",
        "--bin",
        "pogly-cli",
        "--",
        "mcp"
      ]
    }
  }
}
```

## Open Sound Control (OSC) Server

`pogly-cli` can run as a UDP-based OSC listener, allowing you to instantly control your overlay layouts and elements with zero process-spawn overhead. This is perfect for integrations with Stream Deck (via an OSC plugin), TouchOSC, VRChat, or custom script controllers.

To match the native 30Hz server-side update rate, the listener throttles outgoing API updates and automatically deduplicates incoming queues (e.g. merging rapid adjustments so only the latest values are sent within the same tick window).

To start the OSC server:

```
pogly osc --port 9000
```

### Supported OSC Address Routes

| OSC Address | Arguments | Description |
|---|---|---|
| `/pogly/ping` | None | Pings the overlay API to verify connectivity. |
| `/pogly/whoami` | None | Prints the connected API token details and permissions. |
| `/pogly/layouts/set-active`<br>`/pogly/layouts/set_active` | `target` (Int, Long, or String) | Switches active layout (accepts layout ID or layout name). |
| `/pogly/elements/delete` | `id` (Int, Long, or String) | Deletes the element with the specified ID. |
| `/pogly/elements/update/position` | `id` (Int/Long/String), `x` (Int/Long/Float/String), `y` (Int/Long/Float/String) | Moves the element to coordinates `x` and `y`. |
| `/pogly/elements/update/transparency` | `id` (Int/Long/String), `transparency` (Int/Long/Float/String) | Sets element transparency (0-100). |
| `/pogly/elements/update/text` | `id` (Int/Long/String), `text` (String/Int/Float/Bool) | Updates text element text content. |
| `/pogly/elements/update/media/playing` | `id` (Int/Long/String), `playing` (Bool/Int) | Plays or pauses a media element. |
| `/pogly/elements/update/media/volume` | `id` (Int/Long/String), `volume` (Int/Long/Float/String) | Sets media element volume (0-100). |
| `/pogly/elements/update/media/timestamp` | `id` (Int/Long/String), `timestamp` (Int/Long/Float/String) | Seeks media element to timestamp in seconds. |
| `/pogly/elements/update` | `id` (Int/Long/String), `field_name` (String), `value` (Any) | Updates a single field dynamically (e.g. `color`, `textSize`, `alwaysLoaded`). |

Values are automatically coerced to the required type (e.g. float arguments will be converted to integers or booleans where appropriate).

## Building from source

```
cargo build --release
```

Produces `target\release\pogly-cli.exe` (the CLI) and `target\release\pogly.exe` (the launcher). Run the CLI directly during development: `cargo run -p pogly-cli -- <args>`.

To test `version upgrade` against a fork, set `POGLY_RELEASES_REPO=<owner>/<repo>`.

## License

[MIT](LICENSE)
