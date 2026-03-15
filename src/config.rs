use dirs::config_dir;
use log::warn;
use std::path::PathBuf;

fn get_config_dir() -> PathBuf {
    if let Some(dir) = config_dir() {
        return dir;
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata);
        }
        if let Some(localappdata) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(localappdata);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".config");
            return path;
        }
    }

    warn!("Could not determine user config directory, falling back to current directory");
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn data_dir() -> PathBuf {
    let mut path = get_config_dir();
    path.push("game-time-tracker");

    if let Err(e) = std::fs::create_dir_all(&path) {
        warn!(
            "Failed to create data directory at {}: {}",
            path.display(),
            e
        );
    }

    path
}

pub fn bin_dir() -> PathBuf {
    data_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_dir_creation() {
        let dir = data_dir();
        assert!(dir.to_string_lossy().contains("game-time-tracker"));
    }

    #[test]
    fn test_data_dir_is_absolute() {
        let dir = data_dir();
        assert!(dir.is_absolute(), "Data dir should be absolute path");
    }
}
