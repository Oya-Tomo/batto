use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::time::Duration;

use crate::config::lua_engine::LuaRuntime;
use crate::config::types::{AppConfig, UserCommand};
use crate::discovery::types::AppEntry;

fn runtime_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("batto")
}

pub fn socket_path() -> PathBuf {
    runtime_dir().join("batto.sock")
}

pub fn is_daemon_running() -> bool {
    let path = socket_path();
    if !path.exists() {
        return false;
    }
    if let Ok(mut stream) = UnixStream::connect(&path) {
        let _ = stream.write_all(b"ping");
        let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
        let mut buf = [0u8; 8];
        stream.read(&mut buf).is_ok()
    } else {
        let _ = std::fs::remove_file(&path);
        false
    }
}

fn shutdown_daemon() {
    let path = socket_path();
    if !path.exists() {
        return;
    }
    if let Ok(mut stream) = UnixStream::connect(&path) {
        let _ = stream.write_all(b"shutdown");
        // Wait for socket to be released
        for _ in 0..50 {
            std::thread::sleep(Duration::from_millis(50));
            if !path.exists() {
                return;
            }
        }
        // Force cleanup if still hanging
        let _ = std::fs::remove_file(&path);
    }
}

fn spawn_daemon() {
    let dir = runtime_dir();
    std::fs::create_dir_all(&dir).ok();

    let log_file = std::fs::File::create(dir.join("daemon.log")).expect("cannot create log file");

    let exe = std::env::current_exe().expect("cannot find executable");
    let daemon_exe = exe.with_file_name("battod");

    let mut cmd = std::process::Command::new(daemon_exe);
    cmd.stdout(log_file.try_clone().expect("clone log file"))
        .stderr(log_file)
        .stdin(std::process::Stdio::null());

    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    cmd.spawn().expect("failed to start daemon");

    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(50));
        if is_daemon_running() {
            break;
        }
    }
}

pub fn restart_daemon() {
    shutdown_daemon();
    spawn_daemon();
}

pub fn start_daemon() {
    if is_daemon_running() {
        return;
    }
    spawn_daemon();
}

// --- History ---

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct HistoryEntry {
    name: String,
    last_used: u64,
    count: u32,
}

fn history_path() -> PathBuf {
    runtime_dir().join("history.json")
}

fn load_history() -> Vec<HistoryEntry> {
    let path = history_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_history(history: &[HistoryEntry]) {
    if let Ok(json) = serde_json::to_string(history) {
        let _ = std::fs::write(history_path(), json);
    }
}

fn record_launch(history: &mut Vec<HistoryEntry>, name: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if let Some(entry) = history.iter_mut().find(|e| e.name == name) {
        entry.last_used = now;
        entry.count += 1;
    } else {
        history.push(HistoryEntry {
            name: name.to_string(),
            last_used: now,
            count: 1,
        });
    }
}

fn sort_by_history(apps: &mut [AppEntry], history: &[HistoryEntry]) {
    apps.sort_by(|a, b| {
        let a_h = history.iter().find(|h| h.name == a.name);
        let b_h = history.iter().find(|h| h.name == b.name);
        match (a_h, b_h) {
            (Some(a_h), Some(b_h)) => b_h.last_used.cmp(&a_h.last_used),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.name_lower.cmp(&b.name_lower),
        }
    });
}

// --- Data types ---

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DaemonData {
    pub config: AppConfig,
    pub apps: Vec<AppEntry>,
    pub commands: Vec<UserCommand>,
}

pub fn request_all() -> Result<DaemonData, String> {
    let mut stream = UnixStream::connect(socket_path()).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;

    stream.write_all(b"get_all").map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).map_err(|e| e.to_string())?;

    serde_json::from_slice(&buf).map_err(|e| e.to_string())
}

pub fn request_query(name: &str, query: &str) -> Result<Vec<crate::config::types::QueryResult>, String> {
    let mut stream = UnixStream::connect(socket_path()).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;

    let msg = format!("query:{name}:{query}");
    stream.write_all(msg.as_bytes()).map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).map_err(|e| e.to_string())?;

    serde_json::from_slice(&buf).map_err(|e| e.to_string())
}

pub fn request_exec_handler(name: &str, args: &std::collections::HashMap<String, String>) -> Result<Vec<crate::config::types::QueryResult>, String> {
    let mut stream = UnixStream::connect(socket_path()).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;

    let json_args = serde_json::to_string(args).unwrap_or_default();
    let msg = format!("exec_handler:{name}:{json_args}");
    stream.write_all(msg.as_bytes()).map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).map_err(|e| e.to_string())?;

    serde_json::from_slice(&buf).map_err(|e| e.to_string())
}

pub fn request_rescan() -> Result<(), String> {
    let mut stream = UnixStream::connect(socket_path()).map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream.write_all(b"rescan").map_err(|e| e.to_string())?;
    let mut buf = [0u8; 16];
    stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
    if &buf[..n] == b"ok" {
        Ok(())
    } else {
        Err("rescan failed".into())
    }
}

// --- Daemon server ---

pub fn run_daemon() {
    let dir = runtime_dir();
    std::fs::create_dir_all(&dir).ok();

    crate::config::load_dotenv();
    let config_path = crate::config::ensure_config();
    let mut lua_runtime = match LuaRuntime::new(&config_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("daemon: failed to parse config: {e}");
            return;
        }
    };
    let output = lua_runtime.output();
    let mut apps = crate::discovery::discover_apps();

    let history = load_history();
    sort_by_history(&mut apps, &history);

    let data = DaemonData {
        config: output.config.clone(),
        apps,
        commands: output.commands.clone(),
    };
    let cache_path = dir.join("cache.json");
    if let Ok(json) = serde_json::to_string(&data) {
        let _ = std::fs::write(&cache_path, json);
    }

    let path = socket_path();
    let mut history = history;
    if path.exists() {
        std::fs::remove_file(&path).ok();
    }
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("daemon: failed to bind socket: {e}");
            return;
        }
    };

    // Rescan writes to a flag file; main thread picks it up
    let rescan_flag = dir.join(".rescan");
    let rescan_flag_clone = rescan_flag.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(300));
        let _ = std::fs::write(&rescan_flag_clone, "1");
    });

    for stream in listener.incoming() {
        // Check rescan flag
        if rescan_flag.exists() {
            let _ = std::fs::remove_file(&rescan_flag);
            crate::config::load_dotenv();
            if lua_runtime.reload(&config_path).is_ok() {
                let out = lua_runtime.output();
                let mut new_apps = crate::discovery::discover_apps();
                let h = load_history();
                sort_by_history(&mut new_apps, &h);
                let data = DaemonData {
                    config: out.config.clone(),
                    apps: new_apps,
                    commands: out.commands.clone(),
                };
                if let Ok(json) = serde_json::to_string(&data) {
                    let _ = std::fs::write(&cache_path, &json);
                }
            }
        }

        match stream {
            Ok(mut stream) => {
                let mut buf = [0u8; 4096];
                if let Ok(n) = stream.read(&mut buf) {
                    let cmd = String::from_utf8_lossy(&buf[..n]).to_string();
                    if cmd == "ping" {
                        let _ = stream.write_all(b"ok");
                    } else if cmd == "shutdown" {
                        let _ = stream.write_all(b"ok");
                        let _ = std::fs::remove_file(&path);
                        break;
                    } else if cmd == "get_all" {
                        if let Ok(data) = std::fs::read(&cache_path) {
                            let _ = stream.write_all(&data);
                        } else {
                            let _ = stream.write_all(b"{}");
                        }
                    } else if cmd == "rescan" {
                        crate::config::load_dotenv();
                        if lua_runtime.reload(&config_path).is_ok() {
                            let out = lua_runtime.output();
                            let mut new_apps = crate::discovery::discover_apps();
                            let h = load_history();
                            sort_by_history(&mut new_apps, &h);
                            let data = DaemonData {
                                config: out.config.clone(),
                                apps: new_apps,
                                commands: out.commands.clone(),
                            };
                            if let Ok(json) = serde_json::to_string(&data) {
                                let _ = std::fs::write(&cache_path, &json);
                                let _ = stream.write_all(b"ok");
                            }
                        }
                    } else if let Some(name) = cmd.strip_prefix("launch:") {
                        record_launch(&mut history, name.trim());
                        save_history(&history);
                        let _ = stream.write_all(b"ok");
                    } else if let Some(rest) = cmd.strip_prefix("query:") {
                        // Format: query:<name>:<text>
                        let (name, text) = rest.split_once(':').unwrap_or((rest, ""));
                        let results = lua_runtime.query(name, text);
                        if let Ok(json) = serde_json::to_string(&results) {
                            let _ = stream.write_all(json.as_bytes());
                        } else {
                            let _ = stream.write_all(b"[]");
                        }
                    } else if let Some(rest) = cmd.strip_prefix("exec_handler:") {
                        // Format: exec_handler:<name>:<json_args>
                        let (name, json_args) = rest.split_once(':').unwrap_or((rest, "{}"));
                        let args: std::collections::HashMap<String, String> =
                            serde_json::from_str(json_args).unwrap_or_default();
                        let results = lua_runtime.exec_handler(name, &args);
                        if let Ok(json) = serde_json::to_string(&results) {
                            let _ = stream.write_all(json.as_bytes());
                        } else {
                            let _ = stream.write_all(b"[]");
                        }
                    } else {
                        let _ = stream.write_all(b"unknown");
                    }
                }
            }
            Err(_) => break,
        }
    }
}
