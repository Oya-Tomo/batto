# Usage

## Starting the launcher

```
batto
```

On first run, `batto` automatically starts `battod` in the background. Subsequent launches are near-instant because the daemon has already scanned and cached your applications.

## Display modes

### App mode (default)

Type to fuzzy-search installed applications. Results appear as a horizontal scrolling icon row with real icons.

| Key | Action |
|---|---|
| Type | Filter apps by name |
| Left / Right | Navigate icon row |
| Enter | Launch selected app |
| Escape | Close launcher |

### Command mode

Type `/` to switch to command mode. Shows user-defined commands as a vertical list.

| Key | Action |
|---|---|
| `/` | Enter command mode |
| Type after `/` | Filter commands by name |
| Up / Down | Navigate command list |
| Enter | Execute selected command |
| Backspace to empty | Return to app mode |

## Configuration

Config file: `~/.config/batto/init.lua`

Generated automatically on first launch with defaults.

For the full Lua API reference, see [Lua API Reference](lua-api.md).

Below are the basic usage examples.

### batto.setup()

```lua
batto.setup({
  window = {
    width = 600,
    list_height = 300,
    icon_size = 48,
    show_name = true,   -- show app name below icon
  },
  keys = {
    accept = "enter",
    close = "escape",
    up = "up",
    down = "down",
    tab_complete = "tab",
  },
})
```

### batto.command()

Define custom commands accessible via `/` mode:

```lua
batto.command({
  name = "search",
  description = "Search the web",
  args = {
    { name = "query", required = true, type = "string" },
  },
  exec = "xdg-open 'https://google.com/search?q={{args}}'",
})
```

Usage in launcher: `/search rust lang` → opens Google for "rust lang".

Argument validation types: `string`, `path`, `url`, `number`.

## Daemon

The daemon (`battod`) runs in the background and:

- Reads `~/.config/batto/.env` for environment variables
- Scans `.desktop` files from `/usr/share/applications/` and `~/.local/share/applications/`
- Resolves icons via freedesktop icon theme
- Parses your Lua config and user commands
- Records launch history for recency-based sorting
- Serves data to the client via Unix socket (`~/.cache/batto/batto.sock`)
- Re-scans every 5 minutes automatically

### Built-in commands

| Command | Description |
|---------|-------------|
| `/reload` | Reload config and `.env` without restarting daemon |
| `/restart` | Restart daemon (picks up env vars from `.env`) |

### Environment variables

Store secrets in `~/.config/batto/.env`:

```
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
NOTION_TOKEN=secret_abc123
```

Use them in Lua plugins via `batto.env()`:

```lua
local url = batto.env("DISCORD_WEBHOOK_URL")
```

### Troubleshooting

```
# Check if running
ls ~/.cache/batto/batto.sock

# Stop
pkill battod

# Logs
cat ~/.cache/batto/daemon.log
```

## Keyboard shortcut (recommended)

Bind `batto` to a global keybinding in your desktop environment or window manager. Example for i3/Sway:

```
bindsym $mod+d exec batto
```
