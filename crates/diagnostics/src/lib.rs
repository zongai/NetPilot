//! Connections, inspector, diagnostics, script boundary (NP-097…NP-108).

#![forbid(unsafe_code)]

mod connection;
mod inspector;
mod logging;
mod probe;
mod report;
mod script;
mod testkit;

pub use connection::{
    ConnectionEvent, ConnectionId, ConnectionManager, ConnectionMeta, ConnectionState,
};
pub use inspector::{InspectorApi, InspectorSnapshot};
pub use logging::{ConnectionLog, LogLevel, LogStore};
pub use probe::{run_probe, ProbeKind, ProbeResult};
pub use report::{DiagnosticsReport, ReportSection};
pub use script::{ScriptError, ScriptLimits, ScriptModule, ScriptSandbox};
pub use testkit::inspector_script_smoke;

pub const CRATE_NAME: &str = "netpilot-diagnostics";
