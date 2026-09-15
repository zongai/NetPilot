# R1 Release Agent Rules

1. Baseline is V5 NP-145..NP-240 plus S11.
2. Do not redesign working architecture during release.
3. Never claim a release gate passed without executing it.
4. Production builds must not contain debug endpoints, test credentials or development flags.
5. Never expose credentials, private keys or sensitive subscription URLs.
6. Preserve user configuration/data during upgrade and uninstall unless documented.
7. TUN/Wintun/UAC/driver operations require human review.
8. Record exact build commands, artifact paths and SHA-256 hashes after artifacts actually exist.
9. NP-264 plus human approval is required before calling the result NetPilot 1.0 Final.
