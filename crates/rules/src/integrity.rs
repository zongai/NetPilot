//! Integrity / signature verification (NP-094).
//! Simple non-crypto checksum for content integrity in tests; real signatures later.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityError {
    Mismatch,
    MissingSignature,
    InvalidFormat,
}

impl std::fmt::Display for IntegrityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mismatch => write!(f, "Mismatch"),
            Self::MissingSignature => write!(f, "MissingSignature"),
            Self::InvalidFormat => write!(f, "InvalidFormat"),
        }
    }
}

impl std::error::Error for IntegrityError {}

/// FNV-1a style fingerprint (not a security signature).
pub fn content_fingerprint(body: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in body.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

pub fn verify_fingerprint(body: &str, expected: &str) -> Result<(), IntegrityError> {
    if expected.trim().is_empty() {
        return Err(IntegrityError::MissingSignature);
    }
    if content_fingerprint(body) != expected.trim() {
        return Err(IntegrityError::Mismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let body = "MATCH,DIRECT\n";
        let fp = content_fingerprint(body);
        verify_fingerprint(body, &fp).unwrap();
        assert!(verify_fingerprint(body, "deadbeef").is_err());
    }
}
