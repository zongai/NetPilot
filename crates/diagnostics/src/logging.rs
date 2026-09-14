//! Structured connection logging (NP-101).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionLog {
    pub level: LogLevel,
    pub connection_id: Option<u64>,
    pub message: String,
}

impl ConnectionLog {
    pub fn info(msg: impl Into<String>) -> Self {
        Self {
            level: LogLevel::Info,
            connection_id: None,
            message: redact(msg.into()),
        }
    }

    pub fn for_conn(id: u64, msg: impl Into<String>) -> Self {
        Self {
            level: LogLevel::Info,
            connection_id: Some(id),
            message: redact(msg.into()),
        }
    }
}

fn redact(mut s: String) -> String {
    for key in ["password=", "uuid=", "token="] {
        if s.to_ascii_lowercase().contains(key) {
            s = "[redacted]".into();
            break;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secrets() {
        let l = ConnectionLog::info("uuid=abc");
        assert_eq!(l.message, "[redacted]");
    }
}
