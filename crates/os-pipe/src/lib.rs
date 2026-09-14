#![allow(unsafe_code)] // Win32 named-pipe FFI only.
//! Windows named-pipe transport (length-prefixed / newline JSON friendly).
//!
//! This crate is the only place that performs Win32 pipe FFI. Higher layers
//! (`netpilot-ipc`, Core, Desktop) stay free of raw handles.

#![cfg_attr(not(windows), allow(dead_code))]

use std::io;
use std::time::Duration;

pub const DEFAULT_PIPE_NAME: &str = "netpilot-core";

/// Full Win32 path form used by Core config validation.
pub fn pipe_path(name: &str) -> String {
    if name.starts_with(r"\\.\pipe\") || name.starts_with(r"//./pipe/") {
        name.to_string()
    } else {
        format!(r"\\.\pipe\{name}")
    }
}

/// Bare pipe name without `\\.\pipe\` prefix (for CreateNamedPipe / .NET client).
pub fn bare_name(path_or_name: &str) -> &str {
    let p = path_or_name;
    if let Some(rest) = p.strip_prefix(r"\\.\pipe\") {
        return rest;
    }
    if let Some(rest) = p.strip_prefix(r"//./pipe/") {
        return rest;
    }
    if let Some(rest) = p.strip_prefix(r"\\.\PIPE\") {
        return rest;
    }
    p
}

#[derive(Debug)]
pub enum PipeTransportError {
    Io(io::Error),
    InvalidInput(&'static str),
    Unsupported(&'static str),
    Timeout,
    Disconnected,
}

impl std::fmt::Display for PipeTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "Io({e})"),
            Self::InvalidInput(m) => write!(f, "InvalidInput({m})"),
            Self::Unsupported(m) => write!(f, "Unsupported({m})"),
            Self::Timeout => write!(f, "Timeout"),
            Self::Disconnected => write!(f, "Disconnected"),
        }
    }
}

impl std::error::Error for PipeTransportError {}

impl From<io::Error> for PipeTransportError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// Platform-neutral session after a client is accepted (or connected).
pub trait PipeSession: Send {
    fn read_line(&mut self, timeout: Duration) -> Result<String, PipeTransportError>;
    fn write_line(&mut self, line: &str) -> Result<(), PipeTransportError>;
    fn close(&mut self);
}

#[cfg(windows)]
mod win;

#[cfg(windows)]
pub use win::{NamedPipeListener, NamedPipeStream};

#[cfg(not(windows))]
pub struct NamedPipeListener;

#[cfg(not(windows))]
impl NamedPipeListener {
    pub fn bind(_pipe_name: &str) -> Result<Self, PipeTransportError> {
        Err(PipeTransportError::Unsupported(
            "named pipes require Windows",
        ))
    }

    pub fn accept(&mut self, _timeout: Duration) -> Result<NamedPipeStream, PipeTransportError> {
        Err(PipeTransportError::Unsupported(
            "named pipes require Windows",
        ))
    }
}

#[cfg(not(windows))]
pub struct NamedPipeStream;

#[cfg(not(windows))]
impl PipeSession for NamedPipeStream {
    fn read_line(&mut self, _timeout: Duration) -> Result<String, PipeTransportError> {
        Err(PipeTransportError::Unsupported(
            "named pipes require Windows",
        ))
    }

    fn write_line(&mut self, _line: &str) -> Result<(), PipeTransportError> {
        Err(PipeTransportError::Unsupported(
            "named pipes require Windows",
        ))
    }

    fn close(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_and_path() {
        assert_eq!(bare_name(r"\\.\pipe\netpilot-core"), "netpilot-core");
        assert_eq!(bare_name("netpilot-core"), "netpilot-core");
        assert!(pipe_path("netpilot-core").ends_with("netpilot-core"));
    }
}
