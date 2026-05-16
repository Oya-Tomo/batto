use mlua::{Lua, LuaSerdeExt, Table};

use super::types::{AppConfig, ArgChoice, CommandArg, QueryResult, UserCommand};

pub struct LuaOutput {
    pub config: AppConfig,
    pub commands: Vec<UserCommand>,
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

    pub fn query(&self, name: &str, text: &str) -> Vec<QueryResult> {
        let handlers: Table = match self.lua.globals().get::<Table>("_batto_handlers") {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let Ok(handler) = handlers.get::<mlua::Function>(name) else {
            return Vec::new();
        };

        match handler.call::<Table>(text) {
            Ok(results) => {
                let mut out = Vec::new();
                for item in results.sequence_values::<Table>() {
                    let Ok(item) = item else { continue };
                    let title: String = item.get("title").unwrap_or_default();
                    let exec: String = item.get("exec").unwrap_or_default();
                    out.push(QueryResult { title, exec });
                }
                out
            }
            Err(e) => {
                eprintln!("batto handler({name}): {e}");
                Vec::new()
            }
        }
    }

    pub fn exec_handler(&self, name: &str, args: &std::collections::HashMap<String, String>) -> Vec<QueryResult> {
        let handlers: Table = match self.lua.globals().get::<Table>("_batto_handlers") {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let Ok(handler) = handlers.get::<mlua::Function>(name) else {
            return Vec::new();
        };

        let lua_args = match self.lua.create_table() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };
        for (k, v) in args {
            let _ = lua_args.set(k.clone(), v.clone());
        }

        match handler.call::<Table>(lua_args) {
            Ok(results) => {
                let mut out = Vec::new();
                for item in results.sequence_values::<Table>() {
                    let Ok(item) = item else { continue };
                    let title: String = item.get("title").unwrap_or_default();
                    let exec: String = item.get("exec").unwrap_or_default();
                    out.push(QueryResult { title, exec });
                }
                out
            }
            Err(e) => {
                eprintln!("batto exec_handler({name}): {e}");
                Vec::new()
            }
        }
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
            let exec: String = args.get("exec").unwrap_or_default();
            let has_handler = args.get::<mlua::Function>("handler").is_ok();

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

            // Store the command metadata
            let cmd_table = lua.create_table()?;
            cmd_table.set("name", name.clone())?;
            cmd_table.set("description", description.clone())?;
            cmd_table.set("args", lua.to_value(&lua_args)?)?;
            cmd_table.set("exec", exec.clone())?;
            cmd_table.set("has_handler", has_handler)?;

            // If handler function is provided, store it in the handlers table
            if let Ok(handler) = args.get::<mlua::Function>("handler") {
                let handlers: Table = lua
                    .globals()
                    .get::<Table>("_batto_handlers")
                    .unwrap_or_else(|_| lua.create_table().unwrap());
                handlers.set(name.clone(), handler)?;
                lua.globals().set("_batto_handlers", handlers)?;
            }

            commands.set(idx, cmd_table)?;
            lua.globals().set("_batto_commands", commands)?;
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
        for pair in tbl.sequence_values::<Table>() {
            let Ok(t) = pair else { continue };
            let name: String = t.get("name").unwrap_or_default();
            let description: String = t.get("description").unwrap_or_default();
            let exec: String = t.get("exec").unwrap_or_default();
            let has_handler: bool = t.get("has_handler").unwrap_or(false);
            let args: Vec<CommandArg> = match t.get::<Table>("args") {
                Ok(at) => {
                    let mut cmd_args = Vec::new();
                    for arg_pair in at.sequence_values::<Table>() {
                        let Ok(at2) = arg_pair else { continue };
                        let choices: Vec<ArgChoice> = match at2.get::<Table>("choices") {
                            Ok(ct) => ct.sequence_values::<Table>()
                                .filter_map(|c| c.ok())
                                .filter_map(|c| {
                                    let n: String = c.get("name").ok()?;
                                    let v: String = c.get("value").ok()?;
                                    Some(ArgChoice { name: n, value: v })
                                })
                                .collect(),
                            Err(_) => Vec::new(),
                        };
                        cmd_args.push(CommandArg {
                            name: at2.get("name").unwrap_or_default(),
                            required: at2.get("required").unwrap_or(false),
                            arg_type: at2.get("type").unwrap_or_else(|_| "string".to_string()),
                            choices,
                        });
                    }
                    cmd_args
                }
                Err(_) => Vec::new(),
            };
            cmds.push(UserCommand { name, description, args, exec, has_handler });
        }
        cmds
    } else {
        Vec::new()
    };

    Ok(LuaOutput { config, commands })
}
