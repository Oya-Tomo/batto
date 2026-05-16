pub mod lua_engine;
pub mod types;

use std::fs;
use std::path::PathBuf;

const DEFAULT_INIT_LUA: &str = include_str!("../default_config.lua");

fn config_dir() -> PathBuf {
    dirs::config_dir().expect("cannot determine config directory").join("batto")
}

pub fn plugins_dir() -> PathBuf {
    config_dir().join("plugins")
}

fn init_lua_path() -> PathBuf {
    config_dir().join("init.lua")
}

pub fn env_path() -> PathBuf {
    config_dir().join(".env")
}

pub fn load_dotenv() {
    let path = env_path();
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            // SAFETY: .env values are user-controlled config, no concurrent access
            unsafe { std::env::set_var(key, value); }
        }
    }
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
