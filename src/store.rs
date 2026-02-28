use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub fn load<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> Result<Option<T>, StoreError> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(path)?;
    let data = serde_json::from_str(&contents)?;
    Ok(Some(data))
}

pub fn save<T: Serialize, P: AsRef<Path>>(data: &T, path: P) -> Result<(), StoreError> {
    let path = path.as_ref();

    let mut tmp_path = path.to_path_buf();
    let file_name = tmp_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("data")
        .to_string();
    tmp_path.set_file_name(format!("{}.tmp", file_name));

    let json = serde_json::to_string_pretty(data)?;
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, path)?;

    Ok(())
}
