# batto

Raycast-like application launcher for Linux, built with Rust and GPUI.

## Quick Start

```
cargo build --release
```

Two binaries are produced:

- `target/release/batto` — the GUI client
- `target/release/battod` — the background daemon

Install both somewhere in your `$PATH`, then bind `batto` to a global keybinding:

```
# i3/Sway example
bindsym $mod+d exec batto
```

That's it. On first run, `batto` auto-starts the daemon and generates a default config at `~/.config/batto/init.lua`.

## Documentation

- [Usage guide](docs/usage.md) — keybindings, modes, configuration, commands
- [Architecture](docs/architecture.md) — daemon/client design, project structure, modules
