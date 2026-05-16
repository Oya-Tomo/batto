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

- Scans `.desktop` files from `/usr/share/applications/` and `~/.local/share/applications/`
- Resolves icons via freedesktop icon theme
- Parses your Lua config and user commands
- Records launch history for recency-based sorting
- Serves data to the client via Unix socket (`~/.cache/batto/batto.sock`)
- Re-scans every 5 minutes automatically

You normally don't need to manage it manually. If needed:

```
# Check if running
ls ~/.cache/batto/batto.sock

# Stop
pkill battod

# Logs
cat ~/.cache/batto/daemon.log

# Force rescan
echo -n "rescan" | socat - UNIX-CONNECT:$HOME/.cache/batto/batto.sock
```

## Keyboard shortcut (recommended)

Bind `batto` to a global keybinding in your desktop environment or window manager. Example for i3/Sway:

```
bindsym $mod+d exec batto
```
