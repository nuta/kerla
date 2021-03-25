use alloc::string::{String, ToString};
use core::fmt;
use core::ops::Deref;

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Path {
    path: str,
}

impl Path {
    pub fn new(path: &str) -> &Path {
        unsafe { &*(path as *const str as *const Path) }
    }

    pub fn as_str(&self) -> &str {
        &self.path
    }

    pub fn is_absolute(&self) -> bool {
        self.path.starts_with('/')
    }

    pub fn components(&self) -> Components<'_> {
        let path = if self.path.starts_with('/') {
            &self.path[1..]
        } else {
            &self.path
        };

        Components { path }
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.path)
    }
}
pub struct Components<'a> {
    path: &'a str,
}

impl<'a> Components<'a> {
    pub fn as_path(&self) -> &Path {
        Path::new(self.path)
    }
}

impl<'a> Iterator for Components<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        if self.path.is_empty() {
            return None;
        }

        let (path_str, next_start) = match self.path.find('/') {
            Some(slash_pos) => (&self.path[..slash_pos], slash_pos + 1),
            None => (self.path, self.path.len()),
        };

        self.path = &self.path[next_start..];
        Some(path_str)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PathBuf {
    path: String,
}

impl PathBuf {
    pub fn new() -> PathBuf {
        PathBuf {
            path: String::new(),
        }
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.path)
    }
}

impl Deref for PathBuf {
    type Target = Path;
    fn deref(&self) -> &Path {
        self.as_path()
    }
}

impl<'a> From<&Path> for PathBuf {
    fn from(path: &Path) -> PathBuf {
        PathBuf {
            path: path.path.to_string(),
        }
    }
}

impl From<String> for PathBuf {
    fn from(path: String) -> PathBuf {
        PathBuf { path }
    }
}

impl From<&str> for PathBuf {
    fn from(path: &str) -> PathBuf {
        PathBuf {
            path: path.to_string(),
        }
    }
}
