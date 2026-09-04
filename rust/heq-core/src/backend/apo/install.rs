use std::env;
use std::fs;
use std::path::PathBuf;

pub const HEQ_FILE_NAME: &str = "heq.txt";
pub const MAIN_CONFIG_NAME: &str = "config.txt";

// TODO(windows): the C# app also reads HKLM\SOFTWARE\EqualizerAPO\InstallPath, which finds an
// install outside Program Files. HEQ_APO_DIR covers that case until the app builds on Windows.
pub fn find_config_dir() -> Option<PathBuf> {
    candidate_roots()
        .into_iter()
        .map(|root| root.join("config"))
        .find(|cfg| cfg.is_dir())
}

fn candidate_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(dir) = env::var("HEQ_APO_DIR") {
        roots.push(PathBuf::from(dir));
    }
    roots.push(PathBuf::from(r"C:\Program Files\EqualizerAPO"));
    roots.push(PathBuf::from(r"C:\Program Files (x86)\EqualizerAPO"));
    roots
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApoDevice {
    pub name: Option<String>,
    pub currently_disabled: bool,
}

impl ApoDevice {
    pub fn all_devices() -> Self {
        ApoDevice {
            name: None,
            currently_disabled: false,
        }
    }

    pub fn is_all(&self) -> bool {
        self.name.is_none()
    }

    pub fn display(&self) -> String {
        let Some(name) = &self.name else {
            return "All devices".to_string();
        };

        let shown = match name.rfind('{') {
            Some(i) if i > 0 => name[..i].trim(),
            _ => name.as_str(),
        };

        if self.currently_disabled {
            format!("{}  (disabled)", shown)
        } else {
            shown.to_string()
        }
    }
}

pub fn scan_devices(config_dir: Option<&PathBuf>) -> Vec<ApoDevice> {
    let mut list = vec![ApoDevice::all_devices()];

    let Some(dir) = config_dir else { return list };
    let Ok(text) = fs::read_to_string(dir.join(MAIN_CONFIG_NAME)) else {
        return list;
    };

    for raw in text.lines() {
        let mut line = raw.trim();
        let commented = line.starts_with('#');
        if commented {
            line = line.trim_start_matches('#').trim();
        }

        if line.len() < 7 || !line[..7].eq_ignore_ascii_case("Device:") {
            continue;
        }

        let name = line[7..].trim();
        if name.is_empty() || name.eq_ignore_ascii_case("all") {
            continue;
        }
        if list
            .iter()
            .any(|d| d.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(name)))
        {
            continue;
        }

        list.push(ApoDevice {
            name: Some(name.to_string()),
            currently_disabled: commented,
        });
    }

    list
}
