//! NP-144: multi-format airport subscription e2e.

use netpilot_subscription::{
    run_subscription_pipeline, FilterRule, MockFetcher, RenameRule, SubscriptionFetcher,
    SubscriptionManager, SubscriptionProfile, FetchRequest, parse_userinfo_header,
};
use std::time::Duration;

#[test]
fn e2e_uri_clash_singbox_and_userinfo() {
    let sub = SubscriptionProfile::new("s1", "Airport", "https://airport.example/sub");
    // URI
    let uri_body = "trojan://secret@node.example:443?security=tls&type=ws#HK-URI\n";
    let r1 = run_subscription_pipeline(&sub, uri_body, &FilterRule::default(), &RenameRule::default());
    assert_eq!(r1.profiles.len(), 1);

    // Clash
    let clash = r#"
proxies:
  - name: "JP-Clash"
    type: vmess
    server: jp.example
    port: 10086
    uuid: 12345678-1234-1234-1234-123456789abc
"#;
    let r2 = run_subscription_pipeline(&sub, clash, &FilterRule::default(), &RenameRule::default());
    assert_eq!(r2.profiles.len(), 1);

    // Sing-box
    let sb = r#"{"outbounds":[{"type":"vless","tag":"SG","server":"sg.example","server_port":443,"uuid":"12345678-1234-1234-1234-123456789abc"}]}"#;
    let r3 = run_subscription_pipeline(&sub, sb, &FilterRule::default(), &RenameRule::default());
    assert_eq!(r3.profiles.len(), 1);

    // Manager + mock fetch + userinfo
    let mut mgr = SubscriptionManager::new();
    mgr.upsert(sub.clone()).unwrap();
    let mut fetcher = MockFetcher::new();
    fetcher.seed("https://airport.example/sub", uri_body, Some("etag-a"));
    let body = mgr
        .update_one("s1", &mut fetcher, Duration::from_secs(5))
        .unwrap();
    assert!(body.contains("trojan://"));
    let info = parse_userinfo_header("upload=1; download=2; total=10; expire=9");
    assert_eq!(info.used(), Some(3));
}
