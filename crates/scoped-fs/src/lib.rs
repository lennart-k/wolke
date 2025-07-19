mod error;
pub use error::Error;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
// Contains no trailing slash
pub struct ScopedPath(String);

impl ScopedPath {
    pub fn new(path: String) -> Self {
        Self(path.trim_end_matches('/').to_owned())
    }
    pub fn with_base(&self, base: &Path) -> PathBuf {
        // TODO: Ensure that we don't break out of base
        base.join(&self.0)
    }

    pub fn join_segment(&self, name: &str) -> Self {
        let mut path = self.clone();
        if !path.0.is_empty() && !path.0.ends_with('/') {
            path.0.push('/');
        }
        let name = name.trim_start_matches('/');
        path.0.push_str(name);
        path
    }

    pub fn is_collection(&self) -> bool {
        self.0.ends_with('/')
    }

    pub fn file_name(&self) -> &str {
        if let Some((_prefix, filename)) = self.0.rsplit_once('/') {
            filename
        } else {
            self.0.as_str()
        }
    }

    pub fn file_extension(&self) -> Option<&str> {
        let filename = self.file_name();
        filename.rsplit_once('.').map(|(_prefix, ext)| ext)
    }
}

impl<'de> Deserialize<'de> for ScopedPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self::new(String::deserialize(deserializer)?))
    }
}
