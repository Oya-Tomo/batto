#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppEntry {
    pub name: String,
    pub name_lower: String,
    pub exec: String,
    pub icon_path: Option<String>,
    pub categories: Vec<String>,
    pub terminal: bool,
    pub source: AppSource,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AppSource {
    DesktopFile,
    Path,
}
