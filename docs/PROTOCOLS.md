# Protocols & transports (S9, NP-109…NP-120)

`netpilot-protocol-common` defines `ProtocolId`, `TransportId`, `Capabilities`,
`ProtocolUri` parsing, `AdapterManager`, and `compatibility_matrix()`.

Per-protocol crates hold config structs only (no live crypto/network in CI).
