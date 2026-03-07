use serde::{de::DeserializeOwned, Serialize};
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

    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e.into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("test_store.json");

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        save(&data, &test_path).unwrap();
        let loaded: TestData = load(&test_path).unwrap().expect("Failed to load data");

        assert_eq!(data, loaded);

        std::fs::remove_file(test_path).ok();
    }

    #[test]
    fn test_load_nonexistent() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("nonexistent_file_12345.json");

        let result: Result<Option<TestData>, _> = load(&test_path);
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_save_and_load_hashmap() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("test_hashmap.json");

        let mut data: HashMap<String, Vec<TestData>> = HashMap::new();
        data.insert(
            "key1".to_string(),
            vec![
                TestData {
                    name: "a".to_string(),
                    value: 1,
                },
                TestData {
                    name: "b".to_string(),
                    value: 2,
                },
            ],
        );

        save(&data, &test_path).unwrap();
        let loaded: HashMap<String, Vec<TestData>> =
            load(&test_path).unwrap().expect("Failed to load data");

        assert_eq!(data, loaded);

        std::fs::remove_file(test_path).ok();
    }

    #[test]
    fn test_load_malformed_json() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("malformed_test.json");

        std::fs::write(&test_path, "{invalid json").ok();

        let result: Result<Option<TestData>, _> = load(&test_path);
        assert!(result.is_err());

        std::fs::remove_file(test_path).ok();
    }

    #[test]
    fn test_save_overwrites_existing() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("overwrite_test.json");

        let data1 = TestData {
            name: "first".to_string(),
            value: 1,
        };
        save(&data1, &test_path).unwrap();

        let data2 = TestData {
            name: "second".to_string(),
            value: 2,
        };
        save(&data2, &test_path).unwrap();

        let loaded: TestData = load(&test_path).unwrap().expect("Failed to load data");
        assert_eq!(loaded.name, "second");
        assert_eq!(loaded.value, 2);

        std::fs::remove_file(test_path).ok();
    }
}
