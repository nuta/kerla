use alloc::string::{String, ToString};
use core::str::FromStr;

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Path<'a> {
    path: &'a str,
}

impl<'a> Path<'a> {
    pub fn new(path: &str) -> Path<'_> {
        Path { path }
    }

    pub fn components(&self) -> Components<'a> {
        let path = if self.path.starts_with('/') {
            &self.path[1..]
        } else {
            &self.path
        };

        Components { path }
    }
}

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
