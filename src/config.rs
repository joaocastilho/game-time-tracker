use dirs::config_dir;
use log::warn;
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    let mut path = config_dir().unwrap_or_else(|| {
        warn!("Could not determine user config directory, falling back to current directory");
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });

    path.push("game-time-tracker");

    if !path.exists() {
        if let Err(e) = std::fs::create_dir_all(&path) {
            warn!(
                "Failed to create data directory at {}: {}",
                path.display(),
                e
            );
        }
    }

    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_dir_creation() {
        let dir = data_dir();
        assert!(dir.exists(), "Data directory should exist");
        assert!(dir.is_dir(), "Data directory should be a directory");
        assert!(dir.to_string_lossy().contains("game-time-tracker"));
    }

    #[test]
    fn test_data_dir_is_absolute() {
        let dir = data_dir();
        assert!(dir.is_absolute(), "Data dir should be absolute path");
    }
}
