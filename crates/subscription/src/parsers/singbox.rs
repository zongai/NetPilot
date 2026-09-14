//! Sing-box outbound / provider JSON subset parser (NP-132).

use std::collections::HashMap;

use super::ParsedNode;

pub fn parse_singbox_json(text: &str) -> Vec<ParsedNode> {
    let mut nodes = Vec::new();
    // Extremely small JSON object scanner for arrays of outbounds.
    for block in extract_objects(text) {
        if let Some(n) = object_to_node(&block) {
            nodes.push(n);
        }
    }
    nodes
}

fn extract_objects(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut start = None;
    for (i, c) in text.char_indices() {
        match c {
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s) = start.take() {
                        out.push(text[s..=i].to_string());
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn json_str(obj: &str, key: &str) -> Option<String> {
    let patterns = [format!("\"{key}\":\""), format!("\"{key}\": \"")];
    for pat in &patterns {
        if let Some(i) = obj.find(pat) {
            let rest = &obj[i + pat.len()..];
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
    }
    // number
    let pat = format!("\"{key}\":");
    if let Some(i) = obj.find(&pat) {
        let rest = obj[i + pat.len()..].trim_start();
        let end = rest
            .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
            .unwrap_or(rest.len());
        let v = rest[..end].trim().trim_matches('"');
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    None
}

fn object_to_node(obj: &str) -> Option<ParsedNode> {
    let ty = json_str(obj, "type")?.to_ascii_lowercase();
    if matches!(ty.as_str(), "direct" | "block" | "dns" | "selector" | "urltest") {
        return None;
    }
    let server = json_str(obj, "server")?;
    let port: u16 = json_str(obj, "server_port")
        .or_else(|| json_str(obj, "port"))?
        .parse()
        .ok()?;
    let name = json_str(obj, "tag").unwrap_or_else(|| ty.clone());
    let mut params = HashMap::new();
    if let Some(t) = json_str(obj, "transport") {
        params.insert("transport".into(), t);
    }
    if let Some(t) = json_str(obj, "tls") {
        params.insert("tls".into(), t);
    }
    Some(ParsedNode {
        name,
        protocol: ty,
        server,
        port,
        password: json_str(obj, "password"),
        uuid: json_str(obj, "uuid"),
        params,
        source_format: "singbox-json",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_outbound() {
        let j = r#"{"outbounds":[{"type":"trojan","tag":"t1","server":"s.example","server_port":443,"password":"x"}]}"#;
        let nodes = parse_singbox_json(j);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, "t1");
        assert_eq!(nodes[0].port, 443);
    }
}
