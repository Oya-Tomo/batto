pub mod lua_engine;
pub mod types;

use std::fs;
use std::path::PathBuf;

const DEFAULT_INIT_LUA: &str = include_str!("../default_config.lua");

fn config_dir() -> PathBuf {
    dirs::config_dir().expect("cannot determine config directory").join("batto")
}

fn init_lua_path() -> PathBuf {
    config_dir().join("init.lua")
}

pub fn ensure_config() -> PathBuf {
    let path = init_lua_path();
    if !path.exists() {
        let dir = config_dir();
        fs::create_dir_all(&dir).expect("cannot create config directory");
        fs::write(&path, DEFAULT_INIT_LUA).expect("cannot write default config");
    }
    path
}

pub fn load_config() -> types::AppConfig {
    let path = ensure_config();
    lua_engine::parse_config(&path)
        .map(|out| out.config)
        .unwrap_or_else(|e| {
            eprintln!("warning: failed to parse config: {e}");
            types::AppConfig::default()
        })
}

pub fn load_config_and_commands() -> (types::AppConfig, Vec<types::UserCommand>) {
    let path = ensure_config();
    lua_engine::parse_config(&path)
        .map(|out| (out.config, out.commands))
        .unwrap_or_else(|e| {
            eprintln!("warning: failed to parse config: {e}");
            (types::AppConfig::default(), Vec::new())
        })
}
