# Architecture

batto uses a daemon/client architecture with two independent binaries:

```
batto (client)                battod (daemon)
─────────────────────         ─────────────────────
GPUI window rendering         Lua config parsing (mlua)
Fuzzy search (nucleo)         .desktop file scanning
Keyboard input handling       Icon theme resolution
App/command launching         History recording
Mode-switching UI             Cache management
                              Unix socket IPC server
                              5-min periodic rescan
```

The client is ephemeral — it connects to the daemon, gets data, shows the UI, and exits. The daemon is a long-running background process with no GPUI dependency.

## Data flow

```
battod starts
  → parse init.lua (mlua): config + user commands
  → scan .desktop files (freedesktop-desktop-entry)
  → resolve icons (freedesktop icon theme lookup)
  → sort apps by launch history
  → serialize DaemonData { config, apps, commands } to cache.json
  → listen on ~/.cache/batto/batto.sock

batto starts
  → connect to socket → "get_all"
  → deserialize DaemonData { config, apps, commands }
  → open GPUI window with data
  → on launch: notify daemon for history → exit
  → on Escape: exit
```

## Project structure

```
src/
├── lib.rs                  # Shared library root (re-exports all modules)
├── app.rs                  # BattoApp: GPUI Render impl, mode switching
├── daemon.rs               # IPC, socket client/server, history, DaemonData
├── default_config.lua      # Default init.lua template (include_str!'d)
├── bin/
│   ├── batto.rs            # Client entry point (GPUI window)
│   └── battod.rs           # Daemon entry point (pure Rust, no GPUI)
├── config/
│   ├── mod.rs              # ensure_config(), load_config_and_commands()
│   ├── lua_engine.rs       # mlua runtime: batto.setup() + batto.command()
│   └── types.rs            # AppConfig, WindowConfig, KeyConfig, UserCommand
├── commands/
│   ├── mod.rs              # Command trait + CommandRegistry
│   ├── app_launch.rs       # App launch: exec sanitization, Terminal=true handling
│   └── dispatch.rs         # Query parser → command dispatch (unused)
├── discovery/
│   ├── mod.rs              # discover_apps(): scan + sort by history
│   ├── desktop.rs          # .desktop file scanner
│   ├── icon.rs             # Icon theme resolution (SVG/PNG lookup)
│   └── types.rs            # AppEntry, AppSource
└── search/
    ├── mod.rs
    └── fuzzy.rs            # nucleo-matcher wrapper, fuzzy_match()
```

## Key modules

### `app.rs` — UI state (client only)

`BattoApp` holds all UI state: query string, mode (App/Command), selected index, filtered results, full app/command lists, config. Implements `gpui::Render` with `on_key_down` for keyboard input.

Two display modes:
- **App mode** (default): horizontal scrolling icon row with real icons (SVG/PNG)
- **Command mode** (`/` prefix): vertical command list with descriptions

GPUI has no built-in text input widget. All text entry is handled via `KeyDownEvent`.

### `daemon.rs` — IPC layer + history (shared)

Socket protocol is plaintext commands:

| Command | Response |
|---|---|
| `ping` | `ok` |
| `get_all` | JSON `DaemonData { config, apps, commands }` |
| `rescan` | `ok` (reloads .env + Lua config and updates cache) |
| `launch:<name>` | `ok` (records in history) |
| `shutdown` | `ok` (exits daemon) |

History is persisted to `~/.cache/batto/history.json`. Apps are sorted by recency on each scan.

On startup and rescan, the daemon reads `~/.config/batto/.env` and sets environment variables before parsing Lua config. This allows `batto.env()` in plugins to access secrets without shell profile dependencies.

### `config/` — Lua configuration (daemon only at runtime)

- `lua_engine.rs`: Creates an `mlua::Lua` runtime, exposes `batto.setup()` and `batto.command()` as Lua functions.
- `mlua::Lua` is `!Send` — parsed once at daemon startup and discarded.
- `init.lua` is auto-generated at `~/.config/batto/init.lua` on first run.
- `.env` file at `~/.config/batto/.env` is loaded on startup and rescan via `load_dotenv()`.

### `discovery/` — Application scanning + icon resolution (daemon only at runtime)

Scans `/usr/share/applications/` and `~/.local/share/applications/` for `.desktop` files. Skips `NoDisplay=true` and `Hidden=true` entries. Deduplicates by lowercase name.

Icon resolution (`icon.rs`):
1. Absolute path → use directly
2. Icon name → search icon theme chain (GTK settings → inherited themes → hicolor)
3. Prefer SVG (scalable), fall back to PNG (largest first)
4. Final fallback: `/usr/share/pixmaps/`
5. No match → 1-letter placeholder

### `search/fuzzy.rs` — Fuzzy matching (client only)

Wraps `nucleo-matcher`. Matches against `AppEntry::name_lower`. Returns top 20 results sorted by score. Empty query returns first 20 apps.

## Build

```
cargo build --release
```

Produces `target/release/batto` and `target/release/battod`. GPUI requires a running display server (X11 or Wayland).

Install with:

```
cargo install --path .
```

## Key dependencies

| Crate | Version | Purpose |
|---|---|---|
| `gpui` | 0.2.2 | UI framework (Zed's GPU-accelerated UI) |
| `mlua` | 0.11 | Lua 5.4 runtime for config parsing |
| `nucleo-matcher` | 0.3 | Fuzzy string matching |
| `freedesktop-desktop-entry` | 0.8.1 | .desktop file parsing |
| `dirs` | 6 | XDG directory paths |
| `serde` / `serde_json` | 1 | Serialization for IPC and cache |
| `libc` | 0.2 | `setsid` for daemon process detachment |

## Known limitations

- **No IME support**: Text input is via `on_key_down` only. CJK input methods won't work.
