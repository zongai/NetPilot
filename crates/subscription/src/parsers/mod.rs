//! Format parsers (NP-130…NP-132).

mod clash;
mod singbox;
mod uri;

pub use clash::parse_clash_yaml;
pub use singbox::parse_singbox_json;
pub use uri::parse_uri_list;

use std::collections::HashMap;

/// Intermediate node before ProxyProfile normalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedNode {
    pub name: String,
    pub protocol: String,
    pub server: String,
    pub port: u16,
    pub password: Option<String>,
    pub uuid: Option<String>,
    pub params: HashMap<String, String>,
    pub source_format: &'static str,
}
