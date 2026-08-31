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
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let data = serde_json::from_str(&contents)?;
    Ok(Some(data))
}

pub fn save<T: Serialize, P: AsRef<Path>>(data: &T, path: P) -> Result<(), StoreError> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("data")
        .to_string();

    // Use a unique temp name to avoid TOCTOU races between concurrent
    // tracker + UI writers saving the same file (e.g. sessions.json).
    let tmp_path = {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let cnt = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mut p = path.to_path_buf();
        p.set_file_name(format!(
            "{}.tmp.{}-{}-{}",
            file_name,
            std::process::id(),
            nanos,
            cnt
        ));
        p
    };

    let json = serde_json::to_string_pretty(data)?;

    // Write + fsync the temp file so a power-loss does not leave a torn write.
    let write_res: Result<(), std::io::Error> = (|| {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp_path)?;
        f.write_all(json.as_bytes())?;
        f.flush()?;
        f.sync_all()?;
        Ok(())
    })();
    if let Err(e) = write_res {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e.into());
    }

    let result = std::fs::rename(&tmp_path, path);

    if let Err(e) = result {
        let is_cross_device = e.kind() == std::io::ErrorKind::CrossesDevices;

        if is_cross_device {
            // Copy to a second temp in the destination directory then atomic
            // rename, so a crash mid-copy does not truncate the original file.
            let mut tmp2 = path.to_path_buf();
            let nanos2 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            tmp2.set_file_name(format!(
                "{}.tmp2.{}-{}-{:?}",
                file_name,
                std::process::id(),
                nanos2,
                std::thread::current().id()
            ));
            std::fs::copy(&tmp_path, &tmp2)?;
            if let Ok(f) = std::fs::File::open(&tmp2) {
                let _ = f.sync_all();
            }
            let res2 = std::fs::rename(&tmp2, path);
            let _ = std::fs::remove_file(&tmp_path);
            if let Err(e2) = res2 {
                let _ = std::fs::remove_file(&tmp2);
                return Err(e2.into());
            }
        } else {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e.into());
        }
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
