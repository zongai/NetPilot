//! Network diagnostics probes (NP-103).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeKind {
    Dns,
    TcpConnect,
    HttpGet,
    IcmpPing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeResult {
    pub kind: ProbeKind,
    pub target: String,
    pub ok: bool,
    pub detail: String,
    /// Synthetic latency for mock probes (ms).
    pub latency_ms: u32,
}

/// Deterministic mock probes (no real network in unit tests).
pub fn run_probe(kind: ProbeKind, target: &str) -> ProbeResult {
    let target = target.to_string();
    match kind {
        ProbeKind::Dns => ProbeResult {
            kind,
            target,
            ok: true,
            detail: "mock dns ok".into(),
            latency_ms: 5,
        },
        ProbeKind::TcpConnect => ProbeResult {
            kind,
            target,
            ok: true,
            detail: "mock tcp ok".into(),
            latency_ms: 12,
        },
        ProbeKind::HttpGet => ProbeResult {
            kind,
            target,
            ok: true,
            detail: "mock http 200".into(),
            latency_ms: 40,
        },
        ProbeKind::IcmpPing => ProbeResult {
            kind,
            target,
            ok: true,
            detail: "mock icmp ok".into(),
            latency_ms: 8,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_ok() {
        assert!(run_probe(ProbeKind::Dns, "example.com").ok);
    }
}
