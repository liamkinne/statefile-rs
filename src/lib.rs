use serde::de::DeserializeOwned;
use serde::Serialize;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct WriteGuard<'a, T: Serialize + DeserializeOwned + Default> {
    guard: RwLockWriteGuard<'a, T>,
    path: PathBuf,
}

impl<'a, T: Serialize + DeserializeOwned + Default> Drop for WriteGuard<'a, T> {
    fn drop(&mut self) {
        let json = serde_json::to_string_pretty(&*self.guard).unwrap();
        // write the data back to the file
        let path = self.path.clone();
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        file.write_all(json.as_bytes()).unwrap();
        file.flush().unwrap();
    }
}

impl<'a, T: Serialize + DeserializeOwned + Default> std::ops::Deref for WriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, T: Serialize + DeserializeOwned + Default> std::ops::DerefMut for WriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

/// A state file.
///
/// This provides strongly typed access to a JSON file wrapped in a `RwLock`
/// that writes to disk once write access is dropped.
pub struct File<T: Serialize + DeserializeOwned + Default> {
    data: RwLock<T>,
    path: PathBuf,
}

impl<T: Serialize + DeserializeOwned + Default> File<T> {
    pub async fn new(path: impl AsRef<Path> + Copy) -> Result<Self, Box<dyn Error>> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let data = if contents.is_empty() {
            T::default()
        } else {
            serde_json::from_str(&contents)?
        };

        let data = RwLock::new(data);

        let path = path.as_ref().to_path_buf();

        Ok(File { data, path })
    }

    /// Locks this state file with shared read access, causing the current task
    /// to yield until the lock has been acquired.
    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.data.read().await
    }

    /// Locks this state file with exclusive write access, causing the current
    /// task to yield until the lock has been acquired.
    pub async fn write(&self) -> WriteGuard<'_, T> {
        WriteGuard {
            guard: self.data.write().await,
            path: self.path.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::fs;

    #[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
    struct TestData {
        field1: String,
        field2: u32,
    }

    #[tokio::test]
    async fn test_file_create_and_write() {
        let test_path = "test_file_create_and_write.json";
        let file = File::<TestData>::new(test_path).await.unwrap();

        let mut write_guard = file.write().await;
        write_guard.field1 = String::from("Test String");
        write_guard.field2 = 42;
        drop(write_guard); // Forces the Drop trait to be called, data should be written to the file

        let mut file_content = String::new();
        std::fs::File::open(test_path)
            .unwrap()
            .read_to_string(&mut file_content)
            .unwrap();

        assert_eq!(
            file_content,
            r#"{
  "field1": "Test String",
  "field2": 42
}"#
        );

        let _ = fs::remove_file(test_path); // Clean up test file
    }

    #[tokio::test]
    async fn test_file_read() {
        let test_path = "test_file_read.json";
        std::fs::write(test_path, r#"{"field1":"Test String","field2":42}"#).unwrap(); // Write initial data

        let file = File::<TestData>::new(test_path).await.unwrap();
        let read_guard = file.read().await;

        assert_eq!(read_guard.field1, "Test String");
        assert_eq!(read_guard.field2, 42);

        let _ = fs::remove_file(test_path); // Clean up test file
    }

    #[tokio::test]
    async fn test_file_read_default() {
        let test_path = "test_file_read_default.json";
        std::fs::write(test_path, "").unwrap(); // Write empty file

        let file = File::<TestData>::new(test_path).await.unwrap();
        let read_guard = file.read().await;

        // Check default values
        assert_eq!(read_guard.field1, "");
        assert_eq!(read_guard.field2, 0);

        let _ = fs::remove_file(test_path); // Clean up test file
    }
}
