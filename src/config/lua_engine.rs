use mlua::{Lua, LuaSerdeExt, Table};

use super::types::{AppConfig, CommandArg, UserCommand};

pub struct LuaOutput {
    pub config: AppConfig,
    pub commands: Vec<UserCommand>,
}

pub fn parse_config(config_path: &std::path::Path) -> Result<LuaOutput, Box<dyn std::error::Error>> {
    let lua = Lua::new();
    let globals = lua.globals();

    globals.set("batto", lua.create_table()?)?;
    let batto_table: Table = globals.get("batto")?;

    batto_table.set("setup", lua.create_function(|lua, args: Table| {
        lua.globals().set("_batto_config", args)?;
        Ok(())
    })?)?;

    batto_table.set("use", lua.create_function(|_lua, _args: String| Ok(()))?)?;

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
                        cmd_args.push(CommandArg {
                            name: t.get("name")?,
                            required: t.get("required").unwrap_or(false),
                            arg_type: t.get("type").unwrap_or_else(|_| "string".to_string()),
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

    let source = std::fs::read_to_string(config_path)?;
    lua.load(&source).exec()?;

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

    Ok(LuaOutput { config, commands })
}
