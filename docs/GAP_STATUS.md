# Gap status

## This wave

| Area | Status |
|------|--------|
| System proxy | WinINET registry + auto on SOCKS (`system_proxy: true`) + IPC |
| VMess AEAD | Header AEAD, session keys, chunk seal |
| SSR | Stream cipher + auth_aes128_md5 framing |
| REALITY | X25519 + HKDF auth + SessionID/key_share hello |
| UDP | SOCKS5 UDP ASSOCIATE + direct relay |
| Process | Windows live path/PID resolver |

## Limits

- Protocol wire formats are compatibility-oriented subsets
- REALITY is auth+fingerprint, not full XTLS-REALITY product stack
- UDP to remote proxy (SS/VMess UDP) still staged
- Some apps ignore WinINET system proxy
