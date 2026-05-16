use mlua::{Lua, LuaSerdeExt, Table};

use super::types::{AppConfig, ArgChoice, CommandArg, QueryHandlerInfo, QueryResult, UserCommand};

pub struct LuaOutput {
    pub config: AppConfig,
    pub commands: Vec<UserCommand>,
    pub query_handlers: Vec<QueryHandlerInfo>,
}

pub struct LuaRuntime {
    lua: Lua,
    output: LuaOutput,
}

impl LuaRuntime {
    pub fn new(config_path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let lua = Lua::new();
        let globals = lua.globals();

        globals.set("batto", lua.create_table()?)?;
        let batto_table: Table = globals.get("batto")?;

        register_batto_apis(&lua, &batto_table)?;

        let source = std::fs::read_to_string(config_path)?;
        lua.load(&source).exec()?;

        let output = extract_output(&lua, &globals)?;
        Ok(Self { lua, output })
    }

    pub fn output(&self) -> &LuaOutput {
        &self.output
    }

    pub fn query(&self, prefix: &str, text: &str) -> Vec<QueryResult> {
        let handlers: Table = match self.lua.globals().get::<Table>("_batto_query_handlers") {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        for pair in handlers.sequence_values::<Table>() {
            let Ok(t) = pair else { continue };
            let Ok(p) = t.get::<String>("prefix") else { continue };
            if p != prefix {
                continue;
            }
            let Ok(handler) = t.get::<mlua::Function>("handler") else { continue };
            match handler.call::<Table>(text) {
                Ok(results) => {
                    let mut out = Vec::new();
                    for item in results.sequence_values::<Table>() {
                        let Ok(item) = item else { continue };
                        let title: String = item.get("title").unwrap_or_default();
                        let exec: String = item.get("exec").unwrap_or_default();
                        out.push(QueryResult { title, exec });
                    }
                    return out;
                }
                Err(e) => {
                    eprintln!("batto on_query({prefix}): {e}");
                    return Vec::new();
                }
            }
        }
        Vec::new()
    }

    pub fn reload(&mut self, config_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        // Re-create Lua to get a clean state
        let lua = Lua::new();
        let globals = lua.globals();

        globals.set("batto", lua.create_table()?)?;
        let batto_table: Table = globals.get("batto")?;
        register_batto_apis(&lua, &batto_table)?;

        let source = std::fs::read_to_string(config_path)?;
        lua.load(&source).exec()?;

        self.output = extract_output(&lua, &globals)?;
        self.lua = lua;
        Ok(())
    }
}

/// Backward-compatible function for one-shot config parsing (used by client fallback)
pub fn parse_config(config_path: &std::path::Path) -> Result<LuaOutput, Box<dyn std::error::Error>> {
    let runtime = LuaRuntime::new(config_path)?;
    Ok(runtime.output)
}

fn register_batto_apis(lua: &Lua, batto_table: &Table) -> Result<(), Box<dyn std::error::Error>> {
    batto_table.set("setup", lua.create_function(|lua, args: Table| {
        lua.globals().set("_batto_config", args)?;
        Ok(())
    })?)?;

    batto_table.set("use", lua.create_function(|lua, name: String| {
        let loaded: Table = lua
            .globals()
            .get::<Table>("_batto_loaded_plugins")
            .unwrap_or_else(|_| lua.create_table().unwrap());
        if let Ok(true) = loaded.get::<bool>(name.clone()) {
            return Ok(());
        }
        loaded.set(name.clone(), true)?;
        lua.globals().set("_batto_loaded_plugins", loaded)?;

        let plugin_path = dirs::config_dir()
            .map(|d| d.join("batto").join("plugins").join(&name).join("init.lua"));

        let Some(path) = plugin_path else {
            eprintln!("batto.use: cannot determine plugin path for '{name}'");
            return Ok(());
        };

        if !path.exists() {
            eprintln!("batto.use: plugin not found: {}", path.display());
            return Ok(());
        }

        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("batto.use: failed to read {}: {e}", path.display());
                return Ok(());
            }
        };

        if let Err(e) = lua.load(&source).set_name(&format!("plugins/{name}/init.lua")).exec() {
            eprintln!("batto.use: error in plugin '{name}': {e}");
        }

        Ok(())
    })?)?;

    batto_table.set("fetch", lua.create_function(|_lua, (url, opts): (String, Option<Table>)| {
        let mut cmd = std::process::Command::new("curl");
        cmd.arg("-s").arg("--max-time").arg("5");

        if let Some(opts) = opts {
            if let Ok(method) = opts.get::<String>("method") {
                cmd.arg("-X").arg(&method);
            }
            if let Ok(headers) = opts.get::<Table>("headers") {
                for pair in headers.pairs::<String, String>() {
                    if let Ok((k, v)) = pair {
                        cmd.arg("-H").arg(format!("{k}: {v}"));
                    }
                }
            }
            if let Ok(body) = opts.get::<String>("body") {
                cmd.arg("--data").arg(&body);
            }
        }

        cmd.arg(&url);
        let output = cmd.output();
        match output {
            Ok(o) if o.status.success() => {
                Ok(String::from_utf8_lossy(&o.stdout).to_string())
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).to_string();
                Err(mlua::Error::external(format!("fetch failed: {err}")))
            }
            Err(e) => Err(mlua::Error::external(format!("fetch failed: {e}"))),
        }
    })?)?;

    batto_table.set("json_decode", lua.create_function(|lua, s: String| {
        let val: serde_json::Value = serde_json::from_str(&s)
            .map_err(|e| mlua::Error::external(format!("json_decode: {e}")))?;
        lua.to_value(&val)
    })?)?;

    batto_table.set("json_encode", lua.create_function(|lua, val: mlua::Value| {
        let data: serde_json::Value = lua.from_value(val)?;
        serde_json::to_string(&data)
            .map_err(|e| mlua::Error::external(format!("json_encode: {e}")))
    })?)?;

    batto_table.set("env", lua.create_function(|_lua, name: String| {
        match std::env::var(&name) {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None),
        }
    })?)?;

    batto_table.set(
        "command",
        lua.create_function(|lua, args: Table| {
            let name: String = args.get("name")?;
            let description: String = args.get("description").unwrap_or_default();
            let exec: String = args.get("exec")?;

            let lua_args: Vec<CommandArg> = match args.get::<Table>("args") {
                Ok(args_table) => {
                    let mut cmd_args = Vec::new();
                    for pair in args_table.sequence_values::<Table>() {
                        let t = pair?;
                        let choices: Vec<ArgChoice> = match t.get::<Table>("choices") {
                            Ok(ct) => ct.sequence_values::<Table>()
                                .filter_map(|c| c.ok())
                                .filter_map(|c| {
                                    let name: String = c.get("name").ok()?;
                                    let value: String = c.get("value").ok()?;
                                    Some(ArgChoice { name, value })
                                })
                                .collect(),
                            Err(_) => Vec::new(),
                        };
                        cmd_args.push(CommandArg {
                            name: t.get("name")?,
                            required: t.get("required").unwrap_or(false),
                            arg_type: t.get("type").unwrap_or_else(|_| "string".to_string()),
                            choices,
                        });
                    }
                    cmd_args
                }
                Err(_) => Vec::new(),
            };

            let commands: Table = lua
                .globals()
                .get::<Table>("_batto_commands")
                .unwrap_or_else(|_| lua.create_table().unwrap());
            let idx = commands.raw_len() + 1;
            commands.set(idx, lua.to_value(&UserCommand {
                name,
                description,
                args: lua_args,
                exec,
            })?)?;
            lua.globals().set("_batto_commands", commands)?;
            Ok(())
        })?,
    )?;

    batto_table.set(
        "on_query",
        lua.create_function(|lua, args: Table| {
            let prefix: String = args.get("prefix")?;
            let description: String = args.get("description").unwrap_or_default();
            let handler: mlua::Function = args.get("handler")?;

            let handlers: Table = lua
                .globals()
                .get::<Table>("_batto_query_handlers")
                .unwrap_or_else(|_| lua.create_table().unwrap());
            let idx = handlers.raw_len() + 1;

            let entry = lua.create_table()?;
            entry.set("prefix", prefix.clone())?;
            entry.set("description", description.clone())?;
            entry.set("handler", handler)?;
            handlers.set(idx, entry)?;

            lua.globals().set("_batto_query_handlers", handlers)?;

            // Also store metadata in a separate table for easy extraction
            let meta: Table = lua
                .globals()
                .get::<Table>("_batto_query_meta")
                .unwrap_or_else(|_| lua.create_table().unwrap());
            let mi = meta.raw_len() + 1;
            meta.set(mi, lua.to_value(&QueryHandlerInfo { prefix, description })?)?;
            lua.globals().set("_batto_query_meta", meta)?;

            Ok(())
        })?,
    )?;

    Ok(())
}

fn extract_output(lua: &Lua, globals: &mlua::Table) -> Result<LuaOutput, Box<dyn std::error::Error>> {
    let config = if let Ok(tbl) = globals.get::<Table>("_batto_config") {
        let mut config = AppConfig::default();

        if let Ok(window) = tbl.get::<Table>("window") {
            if let Ok(w) = window.get::<u32>("width") {
                config.window.width = w;
            }
            if let Ok(h) = window.get::<u32>("list_height") {
                config.window.list_height = h;
            }
            if let Ok(s) = window.get::<u32>("icon_size") {
                config.window.icon_size = s;
            }
            if let Ok(s) = window.get::<bool>("show_name") {
                config.window.show_name = s;
            }
        }

        if let Ok(keys) = tbl.get::<Table>("keys") {
            if let Ok(v) = keys.get::<String>("accept") {
                config.keys.accept = v;
            }
            if let Ok(v) = keys.get::<String>("close") {
                config.keys.close = v;
            }
            if let Ok(v) = keys.get::<String>("up") {
                config.keys.up = v;
            }
            if let Ok(v) = keys.get::<String>("down") {
                config.keys.down = v;
            }
            if let Ok(v) = keys.get::<String>("tab_complete") {
                config.keys.tab_complete = v;
            }
        }

        config
    } else {
        AppConfig::default()
    };

    let commands = if let Ok(tbl) = globals.get::<Table>("_batto_commands") {
        let mut cmds = Vec::new();
        for pair in tbl.sequence_values::<mlua::Value>() {
            if let Ok(val) = pair {
                if let Ok(cmd) = lua.from_value(val) {
                    cmds.push(cmd);
                }
            }
        }
        cmds
    } else {
        Vec::new()
    };

    let query_handlers = if let Ok(tbl) = globals.get::<Table>("_batto_query_meta") {
        let mut handlers = Vec::new();
        for pair in tbl.sequence_values::<mlua::Value>() {
            if let Ok(val) = pair {
                if let Ok(h) = lua.from_value(val) {
                    handlers.push(h);
                }
            }
        }
        handlers
    } else {
        Vec::new()
    };

    Ok(LuaOutput { config, commands, query_handlers })
}
