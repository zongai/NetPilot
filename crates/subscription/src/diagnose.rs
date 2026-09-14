//! Failure diagnostics (NP-142).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnoseStage {
    Fetch,
    Decode,
    Parse,
    Normalize,
    Apply,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnoseReport {
    pub stage: DiagnoseStage,
    pub message: String,
    pub hint: &'static str,
}

pub fn diagnose_failure(stage: DiagnoseStage, message: impl Into<String>) -> DiagnoseReport {
    let hint = match stage {
        DiagnoseStage::Fetch => "Check URL, network, TLS, and authentication headers",
        DiagnoseStage::Decode => "Body may not be Base64; try plain URI/YAML/JSON",
        DiagnoseStage::Parse => "Unsupported or malformed node line / document",
        DiagnoseStage::Normalize => "Protocol fields missing or not mappable to ProxyProfile",
        DiagnoseStage::Apply => "Runtime rejected profiles; check validation rules",
    };
    DiagnoseReport {
        stage,
        message: message.into(),
        hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_hint() {
        let r = diagnose_failure(DiagnoseStage::Fetch, "timeout");
        assert!(!r.hint.is_empty());
    }
}
