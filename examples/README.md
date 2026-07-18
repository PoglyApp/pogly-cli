# Examples

Small scripts showing how to drive a Pogly overlay from the outside world. All of them assume `pogly` is on your `PATH` with a default overlay configured (`pogly overlay add ...`).

| Script | What it does |
| --- | --- |
| [alert.ps1](alert.ps1) | Spawns a text alert, holds it, fades it out, deletes it — the building block for sub/raid/follow/redemption alerts |
| [sync-layout.ps1](sync-layout.ps1) | Switches the active Pogly layout to match an OBS scene name; skips scenes without a matching layout |
| [set-text.ps1](set-text.ps1) | Rewrites an existing text element — donation goals, death counters, "subs today" |
| [rickroll.sh](rickroll.sh) | Spawns a looping rick roll and teleports it around the canvas for 10 seconds, then cleans up |
| [send-osc.py](send-osc.py) | Sends OSC (Open Sound Control) messages to control layouts and elements over UDP |

## Hooking into Streamer.bot


Any event Streamer.bot can see (Twitch, Kick, YouTube subs/raids/follows/channel points, OBS scene changes) can run these scripts:

1. Create an **Action** for your event trigger.
2. Add a sub-action: **Core → System → Run a Program**.
3. Set:
   - **File/Program**: `powershell.exe`
   - **Arguments**: `-ExecutionPolicy Bypass -File C:\path\to\alert.ps1 -Message "%userName% just subscribed!"`
   - Untick "Wait for exit" so alerts never block your action queue.

Streamer.bot substitutes variables like `%userName%`, `%rewardName%`, and `%obsSceneName%` into the arguments before launching.

Typical pairings:

- **Sub / follow / raid** → `alert.ps1 -Message "%userName% just subscribed!"`
- **Channel point redemption** → `alert.ps1 -Message "%userName% redeemed %rewardName%"` (or get creative — see rickroll.sh)
- **OBS scene changed** → `sync-layout.ps1 -Scene "%obsSceneName%"` and your Pogly scenes follow OBS automatically
- **Counter incremented** → `set-text.ps1 -Id 12 -Text "Deaths: %counter%"`

## Stream Deck

No plugin needed: use the stock **System: Open** action with e.g.

```
pogly layouts set-active --name BRB
```

for one-button scene switches, or point a button at any script above.

## Open Sound Control (OSC)

Alternatively, you can run `pogly` as an OSC listener to receive instant layout and element updates over UDP (e.g. from TouchOSC, Stream Deck OSC plugins, or VRChat):

```
pogly osc --port 9000
```

See [send-osc.py](send-osc.py) for an example sender script.

## Writing your own

Every command supports `--json`, so any language that can spawn a process and parse JSON can orchestrate the overlay:

```powershell
$element = pogly --json elements add text --text "hi" | ConvertFrom-Json
pogly elements delete $element.element.id
```

See the [main README](../README.md#commands) for the full command reference.
