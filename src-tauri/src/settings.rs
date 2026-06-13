use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub language: String,
    pub provider: String,
    pub hotkey: String,
    pub theme: String,  // "light", "dark", "system"
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "auto".to_string(),
            provider: "sensevoice".to_string(),
            hotkey: "Alt+R".to_string(),
            theme: "system".to_string(),
        }
    }
}

impl Settings {
    fn config_path() -> PathBuf {
        let dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("kazamo");
        let _ = std::fs::create_dir_all(&dir);
        dir.join("settings.json")
    }

    pub fn load() -> Self {
        match std::fs::read_to_string(Self::config_path()) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(Self::config_path(), data).map_err(|e| e.to_string())
    }
}
