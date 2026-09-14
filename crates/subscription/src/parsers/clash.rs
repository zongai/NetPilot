//! Clash / Clash Meta / Mihomo YAML subset parser (NP-131).
//! Minimal indentation-free extraction of `proxies:` list entries.

use std::collections::HashMap;

use super::ParsedNode;

pub fn parse_clash_yaml(text: &str) -> Vec<ParsedNode> {
    let mut nodes = Vec::new();
    let mut in_proxies = false;
    let mut current: Option<HashMap<String, String>> = None;

    for line in text.lines() {
        let raw = line.trim_end();
        if raw.starts_with('#') {
            continue;
        }
        if raw.starts_with("proxies:") {
            in_proxies = true;
            continue;
        }
        if in_proxies {
            if !raw.is_empty()
                && !raw.starts_with(' ')
                && !raw.starts_with('-')
                && !raw.starts_with('\t')
                && raw.contains(':')
            {
                // next top-level key
                if let Some(map) = current.take() {
                    if let Some(n) = map_to_node(map) {
                        nodes.push(n);
                    }
                }
                in_proxies = false;
                continue;
            }
            let t = raw.trim();
            if t.starts_with("- ") {
                if let Some(map) = current.take() {
                    if let Some(n) = map_to_node(map) {
                        nodes.push(n);
                    }
                }
                current = Some(HashMap::new());
                let rest = t.trim_start_matches("- ").trim();
                if rest.contains(':') {
                    if let Some((k, v)) = split_kv(rest) {
                        current.as_mut().unwrap().insert(k, v);
                    }
                }
            } else if let Some(ref mut map) = current {
                if let Some((k, v)) = split_kv(t) {
                    map.insert(k, v);
                }
            }
        }
    }
    if let Some(map) = current {
        if let Some(n) = map_to_node(map) {
            nodes.push(n);
        }
    }
    nodes
}

fn split_kv(s: &str) -> Option<(String, String)> {
    let (k, v) = s.split_once(':')?;
    let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
    Some((k.trim().to_string(), v))
}

fn map_to_node(map: HashMap<String, String>) -> Option<ParsedNode> {
    let name = map.get("name")?.clone();
    let protocol = map.get("type")?.to_ascii_lowercase();
    let server = map.get("server")?.clone();
    let port: u16 = map.get("port")?.parse().ok()?;
    let mut params = map.clone();
    params.remove("name");
    params.remove("type");
    params.remove("server");
    params.remove("port");
    params.remove("password");
    params.remove("uuid");
    Some(ParsedNode {
        name,
        protocol,
        server,
        port,
        password: map.get("password").cloned(),
        uuid: map.get("uuid").cloned(),
        params,
        source_format: "clash-yaml",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proxy_list() {
        let y = r#"
proxies:
  - name: "n1"
    type: trojan
    server: a.example
    port: 443
    password: secret
proxy-groups:
  - name: g
"#;
        let nodes = parse_clash_yaml(y);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].server, "a.example");
        assert_eq!(nodes[0].protocol, "trojan");
    }
}
