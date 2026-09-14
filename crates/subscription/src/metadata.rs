//! Subscription-Userinfo header parsing (NP-140).

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SubscriptionUserinfo {
    pub upload: Option<u64>,
    pub download: Option<u64>,
    pub total: Option<u64>,
    pub expire: Option<u64>,
}

impl SubscriptionUserinfo {
    pub fn used(&self) -> Option<u64> {
        match (self.upload, self.download) {
            (Some(u), Some(d)) => Some(u.saturating_add(d)),
            (Some(u), None) => Some(u),
            (None, Some(d)) => Some(d),
            _ => None,
        }
    }
}

/// Parse header like: `upload=1; download=2; total=3; expire=1710000000`
pub fn parse_userinfo_header(value: &str) -> SubscriptionUserinfo {
    let mut info = SubscriptionUserinfo::default();
    for part in value.split(';') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            let v = v.trim().parse::<u64>().ok();
            match k.trim().to_ascii_lowercase().as_str() {
                "upload" => info.upload = v,
                "download" => info.download = v,
                "total" => info.total = v,
                "expire" => info.expire = v,
                _ => {}
            }
        }
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let i = parse_userinfo_header("upload=10; download=20; total=100; expire=1");
        assert_eq!(i.used(), Some(30));
        assert_eq!(i.total, Some(100));
    }
}
