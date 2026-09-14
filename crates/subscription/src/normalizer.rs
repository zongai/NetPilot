//! Convert ParsedNode → ProxyProfile (NP-133).

use netpilot_proxy::{ProtocolKind, ProxyProfile, TransportKind};

use crate::parsers::ParsedNode;
use crate::transport_map::map_transport_params;

pub fn normalize_nodes(nodes: &[ParsedNode], id_prefix: &str) -> Vec<ProxyProfile> {
    nodes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| normalize_one(n, &format!("{id_prefix}-{i}")))
        .collect()
}

fn normalize_one(n: &ParsedNode, id: &str) -> Option<ProxyProfile> {
    let protocol = ProtocolKind::parse(&n.protocol)?;
    let mut profile = ProxyProfile {
        id: id.to_string(),
        name: n.name.clone(),
        protocol,
        transport: None,
        server: n.server.clone(),
        port: n.port,
        password: n.password.clone(),
        uuid: n.uuid.clone(),
        username: None,
        sni: n.params.get("sni").cloned(),
        alpn: n.params.get("alpn").cloned(),
        path: n.params.get("path").cloned(),
        host: n.params.get("host").cloned(),
        flow: n.params.get("flow").cloned(),
        network: n
            .params
            .get("network")
            .or_else(|| n.params.get("type"))
            .cloned(),
        tags: vec![n.source_format.to_string()],
    };
    if let Some(t) = map_transport_params(&n.params) {
        profile.transport = Some(t);
    } else if let Some(net) = profile.network.as_deref() {
        profile.transport = TransportKind::parse(net);
    }
    profile.normalize_fields();
    Some(profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn to_profile() {
        let n = ParsedNode {
            name: "n".into(),
            protocol: "trojan".into(),
            server: "h".into(),
            port: 443,
            password: Some("p".into()),
            uuid: None,
            params: HashMap::new(),
            source_format: "test",
        };
        let profiles = normalize_nodes(&[n], "sub");
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].protocol, ProtocolKind::Trojan);
    }
}
