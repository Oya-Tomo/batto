pub mod desktop;
pub mod icon;
pub mod types;

use types::AppEntry;

pub fn discover_apps() -> Vec<AppEntry> {
    let mut entries = desktop::scan_desktop_files();
    entries.sort_by(|a, b| a.name_lower.cmp(&b.name_lower));
    entries
}
