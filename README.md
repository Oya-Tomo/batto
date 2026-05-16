# batto

Raycast-like application launcher for Linux, built with Rust and GPUI.

## Features

- GPU-accelerated UI with fuzzy search
- Desktop application discovery with icons
- Slash commands with argument forms
- Plugin system (Lua)
- Calculator mode (`=2+3`)
- Dynamic query handlers
- `.env` file support for secrets
- Configurable keybindings

## Requirements

- Linux (X11 / Wayland)
- Rust (latest stable)
- curl (for plugin HTTP features)

## Install

```bash
cargo install --path .
```

Installs `batto` and `battod` to `~/.cargo/bin/`. First launch is slightly slower (daemon startup), subsequent launches are instant.

## Set up a hotkey

### GNOME (GUI)

1. Open **Settings** > **Keyboard**
2. Click **Custom Shortcuts** (or **View and Customize Shortcuts** > **Custom Shortcuts**)
3. Click **Add Shortcut**
4. Fill in:
   - **Name**: `batto`
   - **Command**: `batto`
   - **Shortcut**: press your desired key (e.g. `Ctrl+@`)

### GNOME (CLI)

```bash
gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings \
  "['/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/']"

gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/ \
  name 'batto'

gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/ \
  command 'batto'

gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/ \
  binding '<Ctrl>at'
```

### i3 / Sway

```
bindsym $mod+d exec batto
```

### Hyprland

```
bind = CTRL, at, exec, batto
```

## Uninstall

```bash
rm ~/.cargo/bin/batto ~/.cargo/bin/battod
```

## Configuration

Config file: `~/.config/batto/init.lua`
Env file: `~/.config/batto/.env` (for secrets like API tokens)

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

Available APIs: `batto.setup()`, `batto.command()`, `batto.use()`, `batto.fetch()`, `batto.json_decode()`, `batto.json_encode()`, `batto.env()`. See [Lua API Reference](docs/lua-api.md) for details.

## Built-in commands

| Command | Description |
|---------|-------------|
| `/reload` | Reload config and `.env` without restarting daemon |
| `/restart` | Restart daemon (picks up new env vars from `.env`) |

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
