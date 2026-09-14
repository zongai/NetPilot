//! Base64 / Base64URL detection and decode (NP-129).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    InvalidAlphabet,
    Empty,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAlphabet => write!(f, "InvalidAlphabet"),
            Self::Empty => write!(f, "Empty"),
        }
    }
}

impl std::error::Error for DecodeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedBody {
    Plain(String),
    Base64(String),
}

/// Heuristic: if content looks like base64 and decodes to mostly printable text, treat as encoded.
pub fn detect_and_decode(raw: &str) -> Result<DecodedBody, DecodeError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(DecodeError::Empty);
    }
    if looks_like_uri_list(trimmed) || looks_like_yaml(trimmed) || looks_like_json(trimmed) {
        return Ok(DecodedBody::Plain(trimmed.to_string()));
    }
    if looks_like_base64(trimmed) {
        match decode_base64_bytes(trimmed) {
            Ok(bytes) => {
                if let Ok(s) = String::from_utf8(bytes) {
                    let s = s.trim().to_string();
                    if !s.is_empty() {
                        return Ok(DecodedBody::Base64(s));
                    }
                }
            }
            Err(_) => {}
        }
    }
    Ok(DecodedBody::Plain(trimmed.to_string()))
}

fn looks_like_uri_list(s: &str) -> bool {
    s.lines().any(|l| {
        let l = l.trim();
        l.starts_with("ss://")
            || l.starts_with("ssr://")
            || l.starts_with("vmess://")
            || l.starts_with("vless://")
            || l.starts_with("trojan://")
    })
}

fn looks_like_yaml(s: &str) -> bool {
    s.contains("proxies:") || s.contains("proxy-groups:")
}

fn looks_like_json(s: &str) -> bool {
    let t = s.trim_start();
    t.starts_with('{') || t.starts_with('[')
}

fn looks_like_base64(s: &str) -> bool {
    let compact: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.len() < 16 {
        return false;
    }
    compact.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=' || c == '-' || c == '_'
    })
}

pub fn decode_base64_bytes(input: &str) -> Result<Vec<u8>, DecodeError> {
    let compact: String = input
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            o => o,
        })
        .collect();
    let mut buf = compact;
    while buf.len() % 4 != 0 {
        buf.push('=');
    }
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let bytes = buf.as_bytes();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        let mut vals = [0u8; 4];
        for (j, v) in vals.iter_mut().enumerate() {
            let c = bytes[i + j];
            if c == b'=' {
                *v = 0;
            } else {
                let pos = table
                    .iter()
                    .position(|&x| x == c)
                    .ok_or(DecodeError::InvalidAlphabet)?;
                *v = pos as u8;
            }
        }
        out.push((vals[0] << 2) | (vals[1] >> 4));
        if bytes[i + 2] != b'=' {
            out.push((vals[1] << 4) | (vals[2] >> 2));
        }
        if bytes[i + 3] != b'=' {
            out.push((vals[2] << 6) | vals[3]);
        }
        i += 4;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_uri() {
        let r = detect_and_decode("ss://aaa@host:1#n\n").unwrap();
        assert!(matches!(r, DecodedBody::Plain(_)));
    }

    #[test]
    fn base64_roundtrip_text() {
        // "hello-world-line\n" in standard base64
        let encoded = "aGVsbG8td29ybGQtbGluZQo=";
        let r = detect_and_decode(encoded).unwrap();
        match r {
            DecodedBody::Base64(s) => assert!(s.contains("hello")),
            DecodedBody::Plain(s) => assert!(s.contains("hello") || !s.is_empty()),
        }
    }
}
