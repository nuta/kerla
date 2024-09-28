use crate::prelude::*;
use core::fmt;
use core::ops::Deref;

/// A path string reference (akin to `std::path::Path`).
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Path {
    /// A path string. Trailing slashes are removed unless it points to the root
    /// directory (`"/"`).
    // TODO: should we support non-UTF-8 characters?
    path: str,
}

impl Path {
    pub fn new(path: &str) -> &Path {
        let path = if path == "/" {
            path
        } else {
            path.trim_end_matches('/')
        };

        unsafe { &*(path as *const str as *const Path) }
    }

    pub fn as_str(&self) -> &str {
        &self.path
    }

    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }

    pub fn is_absolute(&self) -> bool {
        self.path.starts_with('/')
            && !self
                .components()
                .any(|comp| matches!(comp, ".." | "." | ""))
    }

    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf {
            path: self.path.to_owned(),
        }
    }

    pub fn components(&self) -> Components<'_> {
        let path = if self.path.starts_with('/') {
            &self.path[1..]
        } else {
            &self.path
        };

        Components { path }
    }

    pub fn parent_and_basename(&self) -> Option<(&Path, &str)> {
        if &self.path == "/" {
            return None;
        }

        if let Some(slash_index) = self.path.rfind('/') {
            let parent_dir = if slash_index == 0 {
                Path::new("/")
            } else {
                Path::new(&self.path[..slash_index])
            };

            let basename = &self.path[(slash_index + 1)..];
            Some((parent_dir, basename))
        } else {
            // A relative path without any slashes.
            Some((Path::new("."), &self.path))
        }
    }
}

impl AsRef<Path> for Path {
    fn as_ref(&self) -> &Path {
        self
    }
}

impl AsRef<Path> for str {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.path)
    }
}

/// An iterator of path components (e.g. `a`, `b`, and `c` in `a/b/c`).
pub struct Components<'a> {
    path: &'a str,
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

/// A owned path string (akin to `std::path::PathBuf`).
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

    pub fn pop(&mut self) {
        if let Some((index, _)) = self.path.char_indices().rfind(|(_, ch)| *ch == '/') {
            self.path.truncate(index);
        }
    }

    pub fn push<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref();
        let path_str = if path.as_str() == "/" {
            "/"
        } else {
            path.as_str().trim_end_matches('/')
        };

        if path.is_absolute() {
            self.path = path_str.to_owned();
        } else {
            if self.path != "/" {
                self.path.push('/');
            }
            self.path.push_str(path_str);
        }
    }
}

impl Default for PathBuf {
    fn default() -> PathBuf {
        PathBuf::new()
    }
}

impl Deref for PathBuf {
    type Target = Path;
    fn deref(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<Path> for PathBuf {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl From<&Path> for PathBuf {
    fn from(path: &Path) -> PathBuf {
        PathBuf {
            path: path.path.to_owned(),
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
            path: path.to_owned(),
        }
    }
}
