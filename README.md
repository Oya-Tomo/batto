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
   - **Shortcut**: press your desired key (e.g. `Ctrl+Space`)
   - **Shortcut**: press your desired key (e.g. `Ctrl+Space`)

### GNOME (CLI)

```bash
gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings \
  "['/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/']"

gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/ \
  name 'batto'

gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/ \
  command 'batto'

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

### Plugin API

**Register a command with `exec` template:**

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

**Register a command with `handler` (dynamic results):**

```lua
batto.command({
  name = "gg",
  description = "Google search",
  handler = function(query)
    return {
      { title = "Search: " .. query,
        exec = "xdg-open 'https://google.com/search?q=" .. query .. "'" },
    }
  end,
})
```

**Command with `args` + `handler`:**

```lua
batto.command({
  name = "discord",
  description = "Send Discord message",
  args = {
    { name = "channel", required = true, type = "literal",
      choices = { { name = "General", value = "https://..." } } },
    { name = "message", required = true, type = "string" },
  },
  handler = function(args)
    return {
      { title = "Send: " .. args.message,
        exec = "curl -s -X POST '" .. args.channel .. "' -H 'Content-Type: application/json' -d '{\"content\":\"" .. args.message .. "\"}'" },
    }
  end,
})
```

- `args` + `exec`: arg form shown, template substitution on submit
- `args` + `handler`: arg form shown, handler called with `{name=value}` table on submit
- No `args` + `handler`: handler called with query text on each keystroke (dynamic search)
- No `args` + `exec`: run on submit

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

Secrets like API tokens can be stored in `~/.config/batto/.env`:

```
MY_API_TOKEN=secret123
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
```

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
