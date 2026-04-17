Gulliant Primitive (V1)

A minimal on-chain primitive for verifiable migration of protocol-assigned user state between wallets on Solana.

Scope

This implementation demonstrates:

same-protocol user state migration
protocol-authorized state updates
append-only event logs
snapshot-based migration validation
single-use export authorization

Out of scope:

cross-protocol state migration
identity systems
scoring logic
off-chain verification
Deployment

Program deployed on Solana Devnet:

DyQAWvMytpgRiE2T9bhzSFCe9b9actobfHFB7RVReX7g

Transaction signature:

3b5RsGYJ2BBtr7giVAtXt2AzF7VA9XvhqJgzPvtJMopL7s3MZYYjc4JSMHt1fEPbSETRuokVaW16Ngt9m2SYdrfV

This deployment corresponds to the implementation described in this repository.

Build Verification

The deployed program can be independently verified against the local build artifact.

ProgramData Address:

DhGijfy3UBgfPomqZy1JxDPiVQTZEqM353BwM5VXSmT2

Upgrade Authority:

Hzv7TeyyKSDNpn4tueXGUZvH3FjudHu4pLR19iXFGzsQ

Local build artifact:

target/deploy/gulliant_v1.so

Dumped deployed artifact:

deployed_program.so

SHA-256:

006659e823b90347f1ff783d62f0f374b1e05e8a80329a042794e487699e8e99

Verification steps:

solana program dump DyQAWvMytpgRiE2T9bhzSFCe9b9actobfHFB7RVReX7g deployed_program.so --url devnet
sha256sum target/deploy/gulliant_v1.so deployed_program.so

The SHA-256 values match, confirming that the deployed program corresponds to the locally built artifact.

License

This project is proprietary.

A limited evaluation license is granted only for the Solana Frontier Hackathon.

See [LICENSE](./LICENSE) for full terms.