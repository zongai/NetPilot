//! Wintun feasibility spike (NP-062).
//!
//! Documents integration constraints without linking wintun.dll in CI.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WintunStatus {
    /// Host is not Windows or DLL not present in this build.
    Deferred,
    Available,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WintunFeasibility {
    pub status: WintunStatus,
    pub notes: Vec<&'static str>,
}

impl WintunFeasibility {
    /// Static evaluation for the current build target.
    pub fn evaluate() -> Self {
        let mut notes = vec![
            "Wintun requires Windows 7+ and a signed wintun.dll next to the Core binary",
            "Session model: CreateAdapter → StartSession → read/write ring buffers",
            "NetPilot will load wintun via a thin FFI module in a later NP (not this spike)",
            "MockTunProvider remains the default in unit tests and non-Windows CI",
        ];
        let status = if cfg!(windows) {
            notes.push("Windows target: real adapter open deferred until FFI + privilege work");
            WintunStatus::Deferred
        } else {
            notes.push("Non-Windows host: Wintun unavailable; use MockTunProvider");
            WintunStatus::Unavailable
        };
        Self { status, notes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_has_notes() {
        let f = WintunFeasibility::evaluate();
        assert!(f.notes.len() >= 3);
    }
}
