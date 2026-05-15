use freedesktop_desktop_entry::DesktopEntry;
use std::path::PathBuf;

use super::types::{AppEntry, AppSource};

pub fn scan_desktop_files() -> Vec<AppEntry> {
    let mut entries = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    let dirs: Vec<PathBuf> = vec![
        PathBuf::from("/usr/share/applications"),
        dirs::home_dir()
            .map(|h| h.join(".local/share/applications"))
            .unwrap_or_default(),
    ];

    for dir in &dirs {
        if !dir.exists() {
            continue;
        }
        if let Ok(files) = std::fs::read_dir(dir) {
            for file in files.flatten() {
                let path = file.path();
                if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                    continue;
                }

                if let Ok(entry) = DesktopEntry::from_path(&path, None::<&[&str]>) {
                    if entry.no_display() || entry.hidden() {
                        continue;
                    }

                    let name = match entry.name(&[] as &[&str]) {
                        Some(n) => n.to_string(),
                        None => continue,
                    };
                    let name_lower = name.to_lowercase();

                    if seen_names.contains(&name_lower) {
                        continue;
                    }
                    seen_names.insert(name_lower.clone());

                    let exec = match entry.exec() {
                        Some(e) => e.to_string(),
                        None => continue,
                    };
                    let raw_icon = entry.icon().map(|s| s.to_string());
                    let icon = super::icon::resolve_icon(&raw_icon);
                    let terminal = entry.terminal();
                    let categories = entry
                        .categories()
                        .map(|v| v.into_iter().map(String::from).collect())
                        .unwrap_or_default();

                    entries.push(AppEntry {
                        name,
                        name_lower,
                        exec,
                        icon_path: icon,
                        categories,
                        terminal,
                        source: AppSource::DesktopFile,
                    });
                }
            }
        }
    }

    entries
}
