use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub window: WindowConfig,
    pub keys: KeyConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct WindowConfig {
    pub width: u32,
    pub list_height: u32,
    pub icon_size: u32,
    pub show_name: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct KeyConfig {
    pub accept: String,
    pub close: String,
    pub up: String,
    pub down: String,
    pub tab_complete: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCommand {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub args: Vec<CommandArg>,
    pub exec: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandArg {
    pub name: String,
    #[serde(default)]
    pub required: bool,
    #[serde(rename = "type", default = "default_arg_type")]
    pub arg_type: String,
    #[serde(default)]
    pub choices: Vec<ArgChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgChoice {
    pub name: String,
    pub value: String,
}

fn default_arg_type() -> String {
    "string".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHandlerInfo {
    pub prefix: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub title: String,
    pub exec: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            keys: KeyConfig::default(),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 600,
            list_height: 300,
            icon_size: 48,
            show_name: true,
        }
    }
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            accept: "enter".into(),
            close: "escape".into(),
            up: "up".into(),
            down: "down".into(),
            tab_complete: "tab".into(),
        }
    }
}
