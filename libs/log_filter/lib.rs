#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use log::{warn, Level, Record};

static DEFAULT_LOG_LEVEL: Level = Level::Info;

pub struct LogFilter {
    patterns: Vec<Pattern>,
}

struct Pattern {
    level: Level,
    module_prefix: String,
}

impl LogFilter {
    /// Constructs a log filter with an empty rule. It accepts all logs.
    pub const fn empty() -> LogFilter {
        LogFilter {
            patterns: Vec::new(),
        }
    }

    pub fn new(pattern: &str) -> LogFilter {
        let mut filter = LogFilter::empty();
        filter.overwrite_filter(pattern);
        filter
    }

    pub fn overwrite_filter(&mut self, pattern: &str) {
        self.patterns.clear();
        if pattern.is_empty() {
            return;
        }

        for p in pattern.split(',') {
            let mut parts = p.split('=');
            let (prefix_str, level_str) = match (parts.next(), parts.next()) {
                // "foo=warn"
                (Some(prefix), Some(level)) => (prefix, level),
                // "warn" (default log level)
                (
                    Some(level @ "error")
                    | Some(level @ "warn")
                    | Some(level @ "info")
                    | Some(level @ "debug")
                    | Some(level @ "trace"),
                    None,
                ) => ("", level),
                // "foo" (enable all logs from "foo")
                (Some(prefix), None) => (prefix, "trace"),
                (_, _) => unreachable!(),
            };

            let prefix = prefix_str.strip_prefix("kerla_").unwrap_or(prefix_str);
            let level = match level_str {
                "error" => Level::Error,
                "warn" => Level::Warn,
                "info" => Level::Info,
                "debug" => Level::Debug,
                "trace" => Level::Trace,
                _ => {
                    warn!(
                        "invalid log level: \"{}\", setting
                    \"info\" level",
                        level_str
                    );
                    Level::Info
                }
            };

            self.patterns.push(Pattern {
                level,
                module_prefix: prefix.to_string(),
            });
        }
    }

    pub fn should_print(&self, record: &Record) -> bool {
        let mut longest_match = 0;
        let mut log_level = DEFAULT_LOG_LEVEL;
        if let Some(module_path) = record.module_path() {
            let module_path = module_path.strip_prefix("kerla_").unwrap_or(module_path);
            for pat in &self.patterns {
                if pat.module_prefix.len() >= longest_match
                    && module_path.starts_with(&pat.module_prefix)
                {
                    longest_match = pat.module_prefix.len();
                    log_level = pat.level;
                }
            }
        }

        record.metadata().level() <= log_level
    }
}

#[cfg(test)]
mod tests {
    use log::RecordBuilder;

    use super::*;

    fn build_record(module: &str, level: Level) -> Record {
        RecordBuilder::new()
            .module_path(Some(module))
            .level(level)
            .build()
    }

    fn run(pattern: &str, module: &str, level: Level) -> bool {
        LogFilter::new(pattern).should_print(&build_record(module, level))
    }

    #[test]
    fn test_default_level() {
        assert_eq!(run("", "foo", Level::Error), true);
        assert_eq!(run("", "foo", Level::Warn), true);
        assert_eq!(run("", "foo", Level::Info), true);
        assert_eq!(run("", "foo", Level::Trace), false);
    }

    #[test]
    fn test_simple_pattern() {
        assert_eq!(run("foo=warn", "foo", Level::Warn), true);
        assert_eq!(run("foo=warn", "foo", Level::Info), false);
        assert_eq!(run("foo=warn", "foo", Level::Debug), false);
    }

    #[test]
    fn test_multiple_patterns() {
        assert_eq!(run("foo=warn,bar=trace", "foo", Level::Warn), true);
        assert_eq!(run("foo=warn,bar=trace", "foo", Level::Info), false);
        assert_eq!(run("foo=warn,bar=trace", "foo", Level::Debug), false);
        assert_eq!(run("foo=warn,bar=trace", "bar", Level::Warn), true);
        assert_eq!(run("foo=warn,bar=trace", "bar", Level::Info), true);
        assert_eq!(run("foo=warn,bar=trace", "bar", Level::Debug), true);
    }

    #[test]
    fn test_changing_default_level() {
        assert_eq!(run("foo", "foo", Level::Trace), true);
        assert_eq!(run("foo", "bar", Level::Trace), false);

        assert_eq!(run("warn", "foo", Level::Warn), true);
        assert_eq!(run("warn", "foo", Level::Info), false);
    }
}
