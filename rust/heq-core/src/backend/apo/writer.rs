use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::format;
use super::install::{ApoDevice, HEQ_FILE_NAME, MAIN_CONFIG_NAME};
use crate::backend::Snapshot;

const BACKUP_SUFFIX: &str = ".heq-backup";

pub struct ApoWriter {
    config_dir: PathBuf,
}

#[derive(Debug, Default)]
pub struct IncludeSpot {
    pub exists: bool,
    pub duplicated: bool,
    pub device: Option<String>,
}

#[derive(Debug, Default)]
pub struct IncludeStatus {
    pub ok: bool,
    pub changed: bool,
    pub message: Option<String>,
}

impl ApoWriter {
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        ApoWriter {
            config_dir: config_dir.into(),
        }
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn heq_path(&self) -> PathBuf {
        self.config_dir.join(HEQ_FILE_NAME)
    }

    pub fn main_config_path(&self) -> PathBuf {
        self.config_dir.join(MAIN_CONFIG_NAME)
    }

    pub fn write_eq(&self, s: &Snapshot) -> io::Result<()> {
        let text = if s.bypassed {
            String::new()
        } else {
            format::build_config(s)
        };
        atomic_write(&self.heq_path(), &text)
    }

    // An include that already exists is never moved: APO's own Editor and Peace rewrite
    // config.txt too, and heq cannot tell a wrong position from a deliberate one.
    pub fn find_include(&self) -> IncludeSpot {
        let Ok(text) = fs::read_to_string(self.main_config_path()) else {
            return IncludeSpot::default();
        };

        let lines: Vec<&str> = text.lines().collect();
        let found = indices_of_include(&lines);
        if found.is_empty() {
            return IncludeSpot::default();
        }

        IncludeSpot {
            exists: true,
            duplicated: found.len() > 1,
            device: device_above(&lines, found[0]),
        }
    }

    pub fn ensure_include(&self, device: Option<&ApoDevice>) -> IncludeStatus {
        let text = fs::read_to_string(self.main_config_path()).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();

        if !indices_of_include(&lines).is_empty() {
            return IncludeStatus {
                ok: true,
                ..Default::default()
            };
        }

        let mut updated: Vec<String> = lines
            .iter()
            .filter(|l| {
                let t = l.trim();
                !t.starts_with("# >>> heq") && !t.starts_with("# <<< heq")
            })
            .map(|l| l.to_string())
            .collect();

        let mut at = 0;
        if let Some(device) = device.filter(|d| !d.is_all()) {
            match index_of_device(&updated, device.name.as_deref().unwrap_or_default()) {
                Some(i) => at = i + 1,
                None => {
                    return IncludeStatus {
                        ok: false,
                        changed: false,
                        message: Some(format!(
                            "{} has no active Device line in config.txt.",
                            device.display()
                        )),
                    }
                }
            }
        }

        updated.insert(at, format!("Include: {}", HEQ_FILE_NAME));

        self.ensure_backup();
        let body = updated.join("\r\n") + "\r\n";
        if let Err(e) = atomic_write(&self.main_config_path(), &body) {
            return IncludeStatus {
                ok: false,
                changed: false,
                message: Some(format!("Could not write config.txt: {}", e)),
            };
        }

        IncludeStatus {
            ok: true,
            changed: true,
            message: Some(format!("Added Include: {} to config.txt.", HEQ_FILE_NAME)),
        }
    }

    fn ensure_backup(&self) {
        let main = self.main_config_path();
        let backup = main.with_extension(
            main.extension()
                .and_then(|e| e.to_str())
                .map(|e| format!("{}{}", e, BACKUP_SUFFIX))
                .unwrap_or_else(|| BACKUP_SUFFIX.trim_start_matches('.').to_string()),
        );

        if main.is_file() && !backup.exists() {
            let _ = fs::copy(&main, &backup);
        }
    }
}

fn index_of_device(lines: &[String], name: &str) -> Option<usize> {
    lines.iter().position(|l| {
        let t = l.trim();
        !t.starts_with('#')
            && t.len() >= 7
            && t[..7].eq_ignore_ascii_case("Device:")
            && t[7..].trim().eq_ignore_ascii_case(name)
    })
}

fn device_above(lines: &[&str], index: usize) -> Option<String> {
    lines[..index].iter().rev().find_map(|l| {
        let t = l.trim();
        (!t.starts_with('#') && t.len() >= 7 && t[..7].eq_ignore_ascii_case("Device:"))
            .then(|| t[7..].trim().to_string())
    })
}

fn indices_of_include(lines: &[&str]) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| is_heq_include(l.trim()))
        .map(|(i, _)| i)
        .collect()
}

fn is_heq_include(trimmed: &str) -> bool {
    trimmed.len() >= 8
        && trimmed[..8].eq_ignore_ascii_case("Include:")
        && trimmed[8..]
            .trim()
            .trim_matches('"')
            .eq_ignore_ascii_case(HEQ_FILE_NAME)
}

fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    let tmp = path.with_extension(match path.extension().and_then(|e| e.to_str()) {
        Some(e) => format!("{}.tmp", e),
        None => "tmp".to_string(),
    });

    fs::write(&tmp, content)?;

    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(&tmp, path)?;
            let _ = fs::remove_file(&tmp);
            Ok(())
        }
    }
}
