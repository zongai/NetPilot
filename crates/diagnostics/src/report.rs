//! Diagnostics report model (NP-104).

use crate::probe::{ProbeKind, ProbeResult, run_probe};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportSection {
    pub title: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsReport {
    pub sections: Vec<ReportSection>,
    pub probes: Vec<ProbeResult>,
}

impl DiagnosticsReport {
    pub fn collect_basic() -> Self {
        let probes = vec![
            run_probe(ProbeKind::Dns, "1.1.1.1"),
            run_probe(ProbeKind::TcpConnect, "1.1.1.1:443"),
        ];
        let mut lines = Vec::new();
        for p in &probes {
            lines.push(format!(
                "{:?} {} ok={} {}ms",
                p.kind, p.target, p.ok, p.latency_ms
            ));
        }
        Self {
            sections: vec![ReportSection {
                title: "Connectivity".into(),
                lines,
            }],
            probes,
        }
    }

    pub fn summary(&self) -> String {
        let ok = self.probes.iter().filter(|p| p.ok).count();
        format!("probes_ok={}/{}", ok, self.probes.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_report() {
        let r = DiagnosticsReport::collect_basic();
        assert!(r.summary().contains("probes_ok="));
    }
}
