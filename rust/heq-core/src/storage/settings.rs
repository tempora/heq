use serde::{Deserialize, Serialize};

use super::Preset;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Settings {
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub bypassed: bool,
    #[serde(default = "default_db_range")]
    pub db_range: f64,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
    #[serde(default)]
    pub last_folder: Option<String>,
    #[serde(default)]
    pub last_preset: Option<String>,
    #[serde(default = "default_width")]
    pub window_width: f64,
    #[serde(default = "default_height")]
    pub window_height: f64,
    #[serde(default)]
    pub ab_folder: Option<String>,
    #[serde(default)]
    pub ab_preset: Option<String>,
    #[serde(default = "default_true")]
    pub match_folder_loudness: bool,
    #[serde(default)]
    pub current: Option<Preset>,
}

fn default_db_range() -> f64 {
    18.0
}
fn default_sample_rate() -> f64 {
    48000.0
}
fn default_width() -> f64 {
    1040.0
}
fn default_height() -> f64 {
    660.0
}
fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            device_name: None,
            bypassed: false,
            db_range: default_db_range(),
            sample_rate: default_sample_rate(),
            last_folder: None,
            last_preset: None,
            window_width: default_width(),
            window_height: default_height(),
            ab_folder: None,
            ab_preset: None,
            match_folder_loudness: true,
            current: None,
        }
    }
}
