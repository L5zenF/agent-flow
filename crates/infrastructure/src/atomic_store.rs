use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::Path;

use serde::Serialize;

#[derive(Debug)]
pub enum AtomicStoreError {
    Io(std::io::Error),
    SerializeToml(toml::ser::Error),
}

impl Display for AtomicStoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::SerializeToml(error) => error.fmt(formatter),
        }
    }
}

impl Error for AtomicStoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::SerializeToml(error) => Some(error),
        }
    }
}

impl From<std::io::Error> for AtomicStoreError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<toml::ser::Error> for AtomicStoreError {
    fn from(value: toml::ser::Error) -> Self {
        Self::SerializeToml(value)
    }
}

pub fn write_toml_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), AtomicStoreError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let serialized = toml::to_string_pretty(value)?;
    let temp_path = path.with_extension("toml.tmp");
    std::fs::write(&temp_path, serialized)?;
    std::fs::rename(temp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde::Serialize;

    use crate::atomic_store::write_toml_atomic;

    #[derive(Debug, Serialize)]
    struct TestDoc {
        value: String,
    }

    fn temp_file_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        path.push(format!("proxy-tools-atomic-store-{name}-{stamp}.toml"));
        path
    }

    #[test]
    fn writes_toml_atomically() {
        let path = temp_file_path("writes");
        let doc = TestDoc {
            value: "ok".to_string(),
        };
        write_toml_atomic(&path, &doc).expect("write should succeed");
        let raw = std::fs::read_to_string(&path).expect("file should be readable");
        assert!(raw.contains("value = \"ok\""));
        let _ = std::fs::remove_file(path);
    }
}
