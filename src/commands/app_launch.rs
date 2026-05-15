use std::process::Command as StdCommand;

use crate::discovery::types::AppEntry;

use super::Command;

pub struct AppLaunchCommand {
    pub entries: Vec<AppEntry>,
}

impl Command for AppLaunchCommand {
    fn name(&self) -> &str {
        "open"
    }

    fn description(&self) -> &str {
        "Launch applications"
    }

    fn execute(&self, args: &str) {
        let query = args.trim().to_lowercase();
        let entry = self.entries.iter().find(|e| e.name_lower == query);

        if let Some(entry) = entry {
            launch_app(entry);
        } else if let Some(entry) = self.entries.iter().find(|e| e.name_lower.contains(&query)) {
            launch_app(entry);
        }
    }

    fn completions(&self, query: &str) -> Vec<String> {
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.name_lower.contains(&q))
            .take(10)
            .map(|e| e.name.clone())
            .collect()
    }
}

pub fn launch_app(entry: &AppEntry) {
    let exec = sanitize_exec(&entry.exec);

    if entry.terminal {
        let term = std::env::var("TERMINAL").unwrap_or_else(|_| "xterm".into());
        let _ = StdCommand::new(term)
            .arg("-e")
            .arg("sh")
            .arg("-c")
            .arg(&exec)
            .spawn();
    } else if entry.source == crate::discovery::types::AppSource::Path {
        let _ = StdCommand::new("sh")
            .arg("-c")
            .arg(&exec)
            .spawn();
    } else {
        let parts: Vec<&str> = exec.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }
        let _ = StdCommand::new(parts[0])
            .args(&parts[1..])
            .spawn();
    }
}

fn sanitize_exec(exec: &str) -> String {
    exec.replace("%f", "")
        .replace("%F", "")
        .replace("%u", "")
        .replace("%U", "")
        .replace("%d", "")
        .replace("%D", "")
        .replace("%i", "")
        .trim()
        .to_string()
}
