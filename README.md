# batto

Raycast-like application launcher for Linux, built with Rust and GPUI.

## Features

- GPU-accelerated UI with fuzzy search
- Desktop application discovery with icons
- Slash commands with argument forms
- Plugin system (Lua)
- Calculator mode (`=2+3`)
- Dynamic query handlers (`batto.on_query`)
- Japanese/IME input support
- Configurable keybindings

## Requirements

- Linux (X11 / Wayland)
- Rust (latest stable)
- curl (for plugin HTTP features)

## Install

```bash
make build
sudo make install
make setup
make enable
```

- `make build` — Build release binaries
- `sudo make install` — Install binaries to `/usr/local/bin`
- `make setup` — Set up systemd user service
- `make enable` — Enable and start the daemon (auto-starts on login)

To install to a different prefix:

```bash
sudo make install PREFIX=~/.local
```

## Set up a hotkey

### GNOME (GUI)

1. Open **Settings** > **Keyboard**
2. Click **Custom Shortcuts** (or **View and Customize Shortcuts** > **Custom Shortcuts**)
3. Click **Add Shortcut**
4. Fill in:
   - **Name**: `batto`
   - **Command**: `batto`
   - **Shortcut**: press your desired key (e.g. `Ctrl+Space`)

### GNOME (CLI)

```bash
gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings \
  "['/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/']"

gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/ \
  name 'batto'

gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/ \
  command '/usr/local/bin/batto'

gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/ \
  binding '<Super>space'
```

### i3 / Sway

```
bindsym $mod+d exec batto
```

### Hyprland

```
bind = SUPER, space, exec, batto
```

## Uninstall

```bash
make uninstall
```

Stops the service and removes binaries and the service unit.

## Configuration

Config file: `~/.config/batto/init.lua`

```lua
batto.setup({
  window = {
    width = 600,
    list_height = 300,
    icon_size = 48,
  },
  keys = {
    accept = "enter",
    close = "escape",
    up = "ctrl+k",
    down = "ctrl+j",
    tab_complete = "tab",
  },
})
```

Keybindings use `modifier+key` format: `ctrl+j`, `ctrl+k`, `alt+p`, `shift+tab`, etc.

## Plugins

Place plugins in `~/.config/batto/plugins/<name>/init.lua` and enable them:

```lua
batto.use("discord")
```

### Plugin API

**Register a command:**

```lua
batto.command({
  name = "hello",
  description = "Say hello",
  args = {
    { name = "target", required = true, type = "string" },
    {
      name = "greeting",
      type = "literal",
      choices = {
        { name = "Hello", value = "hello" },
        { name = "Hi", value = "hi" },
      },
    },
  },
  exec = "echo '{{greeting}} {{target}}'",
})
```

Argument types: `string`, `literal` (selectable from choices).

**Dynamic query handler:**

```lua
batto.on_query({
  prefix = "gg",
  description = "Google search",
  handler = function(query)
    return {
      { title = "Search: " .. query,
        exec = "xdg-open 'https://google.com/search?q=" .. query .. "'" },
    }
  end,
})
```

**HTTP request:**

```lua
local body = batto.fetch("https://api.example.com/data")
local body = batto.fetch("https://api.example.com/data", {
  method = "POST",
  headers = { ["Content-Type"] = "application/json" },
  body = '{"key": "value"}',
})
```

**JSON and env:**

```lua
local data = batto.json_decode(body)
local json = batto.json_encode({ key = "value" })
local token = batto.env("MY_API_TOKEN")
```

## Usage

| Action | How |
|--------|-----|
| Open launcher | Run `batto` (bind to a hotkey) |
| Search apps | Type a query |
| Accept result | Enter |
| Close | Escape |
| Navigate list | Ctrl+J / Ctrl+K (configurable) |
| Tab complete | Tab |
| Slash command | Type `/` then command name |
| Calculator | Type `=` then an expression |

## License

MIT
