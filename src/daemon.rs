use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::time::Duration;

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

pub fn start_daemon() {
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

// --- Daemon server ---

pub fn run_daemon() {
    let dir = runtime_dir();
    std::fs::create_dir_all(&dir).ok();

    let (config, commands) = crate::config::load_config_and_commands();
    let mut apps = crate::discovery::discover_apps();

    let history = load_history();
    sort_by_history(&mut apps, &history);

    let data = DaemonData { config, apps, commands };
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

    let rescan_cache = cache_path.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(300));
        let (config, commands) = crate::config::load_config_and_commands();
        let mut apps = crate::discovery::discover_apps();
        let history = load_history();
        sort_by_history(&mut apps, &history);
        let data = DaemonData { config, apps, commands };
        if let Ok(json) = serde_json::to_string(&data) {
            let _ = std::fs::write(&rescan_cache, json);
        }
    });

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buf = [0u8; 256];
                if let Ok(n) = stream.read(&mut buf) {
                    let cmd = String::from_utf8_lossy(&buf[..n]).to_string();
                    if cmd == "ping" {
                        let _ = stream.write_all(b"ok");
                    } else if cmd == "get_all" {
                        if let Ok(data) = std::fs::read(&cache_path) {
                            let _ = stream.write_all(&data);
                        } else {
                            let _ = stream.write_all(b"{}");
                        }
                    } else if cmd == "rescan" {
                        let (config, commands) = crate::config::load_config_and_commands();
                        let mut apps = crate::discovery::discover_apps();
                        let history = load_history();
                        sort_by_history(&mut apps, &history);
                        let data = DaemonData { config, apps, commands };
                        if let Ok(json) = serde_json::to_string(&data) {
                            let _ = std::fs::write(&cache_path, &json);
                            let _ = stream.write_all(b"ok");
                        }
                    } else if let Some(name) = cmd.strip_prefix("launch:") {
                        record_launch(&mut history, name.trim());
                        save_history(&history);
                        let _ = stream.write_all(b"ok");
                    } else {
                        let _ = stream.write_all(b"unknown");
                    }
                }
            }
            Err(_) => break,
        }
    }
}
