use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

use super::{Correction, Preset, PresetRef, Settings};

pub const MIGRATED_FOLDER: &str = "Unsorted";

const CORRECTION_FILE: &str = "_correction";

pub fn root_dir() -> PathBuf {
    if let Ok(dir) = env::var("HEQ_HOME") {
        return PathBuf::from(dir);
    }

    if cfg!(windows) {
        if let Ok(appdata) = env::var("APPDATA") {
            return PathBuf::from(appdata).join("heq");
        }
    }

    let home = env::var("HOME").map(PathBuf::from).unwrap_or_default();

    if cfg!(target_os = "macos") {
        return home.join("Library/Application Support/heq");
    }

    env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join(".config"))
        .join("heq")
}

pub fn preset_dir() -> PathBuf {
    root_dir().join("presets")
}

pub fn settings_path() -> PathBuf {
    root_dir().join("settings.json")
}

pub fn ensure_dirs() -> io::Result<()> {
    fs::create_dir_all(preset_dir())
}

pub fn migrate_loose_presets() -> usize {
    if ensure_dirs().is_err() {
        return 0;
    }

    let loose: Vec<PathBuf> = json_files(&preset_dir());
    if loose.is_empty() {
        return 0;
    }

    let dir = folder_dir(MIGRATED_FOLDER);
    if fs::create_dir_all(&dir).is_err() {
        return 0;
    }

    let mut moved = 0;
    for file in loose {
        let Some(name) = file.file_name() else { continue };
        let target = dir.join(name);

        let done = if target.exists() {
            fs::remove_file(&file) // the folder copy wins
        } else {
            fs::rename(&file, &target)
        };
        if done.is_ok() {
            moved += 1;
        }
    }
    moved
}

// folders

pub fn folder_dir(folder: &str) -> PathBuf {
    preset_dir().join(sanitize(folder))
}

pub fn list_folders() -> Vec<String> {
    let _ = ensure_dirs();

    let mut names: Vec<String> = fs::read_dir(preset_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    names.sort_by_key(|s| s.to_lowercase());
    names
}

pub fn folder_exists(folder: &str) -> bool {
    !folder.trim().is_empty() && folder_dir(folder).is_dir()
}

pub fn create_folder(folder: &str) -> Option<String> {
    let name = sanitize(folder);
    let dir = folder_dir(&name);
    if dir.exists() {
        return None;
    }
    fs::create_dir_all(&dir).ok()?;
    Some(name)
}

pub fn delete_folder(folder: &str) {
    let dir = folder_dir(folder);
    if dir.is_dir() {
        let _ = fs::remove_dir_all(dir);
    }
}

pub fn rename_folder(folder: &str, new_name: &str) -> Option<String> {
    let from = folder_dir(folder);
    let to = folder_dir(new_name);
    if !from.is_dir() || to.exists() || from == to {
        return None;
    }
    fs::rename(&from, &to).ok()?;
    to.file_name()?.to_str().map(str::to_string)
}

// presets

pub fn list_presets(folder: &str) -> Vec<String> {
    if !folder_exists(folder) {
        return Vec::new();
    }

    let mut names: Vec<String> = json_files(&folder_dir(folder))
        .iter()
        .filter_map(|p| p.file_stem()?.to_str().map(str::to_string))
        .filter(|n| !is_reserved(n))
        .collect();

    names.sort_by_key(|s| s.to_lowercase());
    names
}

pub fn path_for(folder: &str, name: &str) -> PathBuf {
    folder_dir(folder).join(format!("{}.json", sanitize(name)))
}

pub fn save(preset: &Preset, folder: &str) -> io::Result<()> {
    fs::create_dir_all(folder_dir(folder))?;
    let name = preset.name.clone().unwrap_or_else(|| "untitled".into());
    write(&path_for(folder, &name), preset)
}

pub fn load(folder: &str, name: &str) -> Option<Preset> {
    read(&path_for(folder, name))
}

pub fn load_ref(r: &PresetRef) -> Option<Preset> {
    load(&r.folder, &r.name)
}

pub fn exists(folder: &str, name: &str) -> bool {
    path_for(folder, name).is_file()
}

pub fn delete(folder: &str, name: &str) {
    let path = path_for(folder, name);
    if path.is_file() {
        let _ = fs::remove_file(path);
    }
}

pub fn rename(folder: &str, name: &str, new_name: &str) -> Option<String> {
    let from = path_for(folder, name);
    let to = path_for(folder, new_name);
    if !from.is_file() || to.exists() || from == to {
        return None;
    }

    let mut p = load(folder, name)?;
    let renamed = sanitize(new_name);
    p.name = Some(renamed.clone());

    write(&to, &p).ok()?;
    let _ = fs::remove_file(from);
    Some(renamed)
}

pub fn move_preset(folder: &str, name: &str, to_folder: &str) -> bool {
    if folder.eq_ignore_ascii_case(to_folder) {
        return false;
    }

    let from = path_for(folder, name);
    if !from.is_file() || fs::create_dir_all(folder_dir(to_folder)).is_err() {
        return false;
    }

    let to = path_for(to_folder, name);
    if to.exists() {
        return false;
    }

    fs::rename(from, to).is_ok()
}

// the folder's correction

pub fn is_reserved(name: &str) -> bool {
    name.eq_ignore_ascii_case(CORRECTION_FILE)
}

fn correction_path(folder: &str) -> PathBuf {
    folder_dir(folder).join(format!("{}.json", CORRECTION_FILE))
}

pub fn load_correction(folder: &str) -> Correction {
    if folder.is_empty() {
        return Correction::default();
    }
    read(&correction_path(folder)).unwrap_or_default()
}

pub fn save_correction(folder: &str, c: &Correction) {
    if folder.is_empty() {
        return;
    }

    let path = correction_path(folder);
    if c.is_empty() {
        let _ = fs::remove_file(path);
        return;
    }

    if fs::create_dir_all(folder_dir(folder)).is_ok() {
        let _ = write(&path, c);
    }
}

// settings

pub fn load_settings() -> Settings {
    read(&settings_path()).unwrap_or_default() // corrupt settings must never block startup
}

pub fn save_settings(s: &Settings) {
    if ensure_dirs().is_ok() {
        let _ = write(&settings_path(), s);
    }
}

pub fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().to_string();

    if cleaned.is_empty() {
        return "untitled".to_string();
    }
    if is_reserved(&cleaned) {
        return format!("{} (preset)", cleaned);
    }
    cleaned
}

fn json_files(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "json"))
        .collect()
}

fn read<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn write<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, text)
}
