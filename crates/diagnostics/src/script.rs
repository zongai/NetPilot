//! Script/module API boundary, sandbox, limits (NP-105…NP-107).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    Denied(&'static str),
    LimitExceeded(&'static str),
    Runtime(String),
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Denied(m) => write!(f, "Denied: {m}"),
            Self::LimitExceeded(m) => write!(f, "LimitExceeded: {m}"),
            Self::Runtime(m) => write!(f, "Runtime: {m}"),
        }
    }
}

impl std::error::Error for ScriptError {}

#[derive(Debug, Clone)]
pub struct ScriptLimits {
    pub max_memory_bytes: usize,
    pub max_cpu_ms: u64,
    pub max_instructions: u64,
    pub allow_network: bool,
}

impl Default for ScriptLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 8 * 1024 * 1024,
            max_cpu_ms: 50,
            max_instructions: 100_000,
            allow_network: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScriptModule {
    pub name: String,
    pub source: String,
}

/// Capability-restricted sandbox boundary (no interpreter embedded yet).
#[derive(Debug, Clone, Default)]
pub struct ScriptSandbox {
    pub limits: ScriptLimits,
}

impl ScriptSandbox {
    pub fn validate_module(&self, module: &ScriptModule) -> Result<(), ScriptError> {
        if module.name.trim().is_empty() {
            return Err(ScriptError::Denied("empty module name"));
        }
        if module.source.len() > self.limits.max_memory_bytes {
            return Err(ScriptError::LimitExceeded("source too large"));
        }
        if (module.source.contains("std::net") || module.source.contains("TcpStream"))
            && !self.limits.allow_network
        {
            return Err(ScriptError::Denied("network not allowed"));
        }
        Ok(())
    }

    /// Placeholder execute: only validates limits.
    pub fn execute(&self, module: &ScriptModule) -> Result<(), ScriptError> {
        self.validate_module(module)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denies_network() {
        let sb = ScriptSandbox::default();
        let m = ScriptModule {
            name: "x".into(),
            source: "TcpStream::connect".into(),
        };
        assert!(sb.execute(&m).is_err());
    }
}
