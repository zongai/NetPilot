# S11 — Airport Subscription Enhancement

Add-on only. Existing NP-001..NP-120 remain unchanged.

Pipeline:
Subscription URL -> HTTP Fetcher -> Decoder -> Parser -> Normalizer
-> Deduplicate/Filter/Rename -> Proxy Groups -> Runtime Apply

Target formats:
- Base64/Base64URL
- ss://, ssr://, vmess://, vless://, trojan://
- Clash / Clash Meta / Mihomo YAML
- Sing-box JSON

Operational requirements:
- bounded timeout and cancellation
- ETag / Last-Modified
- last-known-good rollback
- credential/secret redaction
- deterministic fixtures
