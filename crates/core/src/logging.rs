//! Structured logging helpers (NP-152).
//!
//! Levels + redaction-aware formatting. Never log passwords, tokens, or raw URLs with secrets.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "error" => Some(Self::Error),
            "warn" | "warning" => Some(Self::Warn),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            "trace" => Some(Self::Trace),
            _ => None,
        }
    }
}

static MAX_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Info as u8);

pub fn set_max_level(level: LogLevel) {
    MAX_LEVEL.store(level as u8, Ordering::Relaxed);
}

pub fn max_level() -> LogLevel {
    match MAX_LEVEL.load(Ordering::Relaxed) {
        1 => LogLevel::Error,
        2 => LogLevel::Warn,
        3 => LogLevel::Info,
        4 => LogLevel::Debug,
        _ => LogLevel::Trace,
    }
}

/// Redact query/userinfo from URLs and obvious token patterns.
#[allow(clippy::manual_pattern_char_comparison)]
pub fn redact_secrets(input: &str) -> String {
    let mut out = input.to_string();
    if let Some(at) = out.find('@') {
        if let Some(scheme) = out.find("://") {
            let start = scheme + 3;
            if start < at {
                if let Some(colon) = out[start..at].find(':') {
                    let pass_start = start + colon + 1;
                    out.replace_range(pass_start..at, "***");
                }
            }
        }
    }
    for key in ["token=", "password=", "passwd=", "key=", "auth="] {
        if let Some(i) = out.to_ascii_lowercase().find(key) {
            let val_start = i + key.len();
            let rest = &out[val_start..];
            let end = rest
                .find(|c: char| matches!(c, '&' | ' ' | '"' | '\''))
                .map(|n| val_start + n)
                .unwrap_or(out.len());
            if val_start < end {
                out.replace_range(val_start..end, "***");
            }
        }
    }
    out
}

pub fn log_line(level: LogLevel, target: &str, message: &str) {
    if level as u8 > max_level() as u8 {
        return;
    }
    let safe = redact_secrets(message);
    eprintln!("[{}] [{}] {}", level.as_str(), target, safe);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_userinfo() {
        let s = redact_secrets("trojan://user:s3cret@example.com:443");
        assert!(s.contains("***"));
        assert!(!s.contains("s3cret"));
    }

    #[test]
    fn redact_token_query() {
        let s = redact_secrets("https://x/sub?token=abc123&n=1");
        assert!(s.contains("token=***"));
        assert!(!s.contains("abc123"));
    }

    #[test]
    fn level_parse() {
        assert_eq!(LogLevel::parse("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::parse("nope"), None);
    }
}
