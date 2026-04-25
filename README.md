# Gulliant Primitive (V1)

An on-chain primitive for verifiable migration of protocol-assigned
user state between wallets on Solana.

Character state (NFT-bound) transfers with the asset.  
Player state (wallet-bound) does not.  
The boundary is enforced at the program level.

---

## Deployed Program

Network: Solana Devnet  
Program ID: `DyQAWvMytpgRiE2T9bhzSFCe9b9actobfHFB7RVReX7g`

```bash
solana program show DyQAWvMytpgRiE2T9bhzSFCe9b9actobfHFB7RVReX7g --url devnet
```

---

## Demo

▶ Watch the proof demo  
https://youtube.com/REPLACE_WITH_YOUR_LINK

---

## Reproducible Build

Verified with:

| Tool | Version |
|------|---------|
| Solana CLI | `1.18.26` |
| Rust | `1.79.0 (stable-x86_64-unknown-linux-gnu)` |
| Build command | `cargo build-sbf` |

> Do not use `cargo build-bpf`. It is deprecated in Solana CLI 1.18.x
> and can produce a different binary.

`Cargo.lock` is committed. Dependency resolution is pinned.  
Clone the repository and build without modifying `Cargo.lock`.

```bash
git clone https://github.com/gulliant/gulliant-primitive.git
cd gulliant-primitive
cargo build-sbf
```

Output artifact:

```text
target/deploy/gulliant_v1.so
```

---

## Verification

Download the deployed binary from Devnet and compare it against the
local build:

```bash
solana program dump DyQAWvMytpgRiE2T9bhzSFCe9b9actobfHFB7RVReX7g   deployed_program.so --url devnet

sha256sum target/deploy/gulliant_v1.so deployed_program.so
```

Both lines must output the same SHA-256 hash:

```text
006659e823b90347f1ff783d62f0f374b1e05e8a80329a042794e487699e8e99
```

If the hashes match, the deployed program is identical to the local
build artifact produced from this repository.

---

## Deployment Metadata

| Field | Value |
|------|-------|
| ProgramData Address | `DhGijfy3UBgfPomqZy1JxDPiVQTZEqM353BwM5VXSmT2` |
| Upgrade Authority | `Hzv7TeyyKSDNpn4tueXGUZvH3FjudHu4pLR19iXFGzsQ` |
| Deployment Signature | `3b5RsGYJ2BBtr7giVAtXt2AzF7VA9XvhqJgzPvtJMopL7s3MZYYjc4JSMHt1fEPbSETRuokVaW16Ngt9m2SYdrfV` |

> Upgrade authority is non-null. On-chain guarantees are conditional on
> the upgrade authority not modifying the program after verification.

---

## What This Primitive Enforces

- Append-only user activity log
- Hash-linked entries using SHA-256
- Snapshot-locked migration
- Single-use export authorization
- Protocol-authorized writes only

---

## What This Primitive Does Not Do

- Cross-protocol migration
- Identity system
- Reputation scoring
- Off-chain verification
- Mainnet deployment

---

## Scope

Demonstrated in V1:

- Same-protocol user state migration
- Protocol-authorized state updates
- Append-only event logs
- Snapshot-based migration validation
- Single-use export authorization

---

## Naive Failure Proof

`demo/naive-transfer/` contains a separate minimal program deployed
on Solana Devnet. It demonstrates the failure mode this primitive
prevents: ownership changes while player-bound state remains attached
to the asset.

Naive Program ID:

```text
FSTYFLyyyUAVGz5bak4waMr29gEawSwKeDgw5n1KBZhi
```

```bash
solana program show FSTYFLyyyUAVGz5bak4waMr29gEawSwKeDgw5n1KBZhi --url devnet
```

See:

```text
demo/naive-transfer/README.md
```

---

## Tests

```bash
RUST_LOG=error cargo test -- --nocapture 2>/dev/null
```

Test coverage:

| Test | Validates |
|------|-----------|
| `test_happy_path` | Full migration under valid conditions |
| `test_missing_protocol_signature` | Rejects append without protocol authority |
| `test_snapshot_mismatch` | Rejects migration after post-authorization append |
| `test_replay_attempt` | Rejects reuse of consumed authorization |
| `test_append_only_invariant` | Rejects overwrite of an existing log entry |

---

## Presentation

Current deck:

```text
docs/gulliant-deck.pdf
```

Archived previous deck:

```text
docs/archive/gulliant-primitive-deck.pdf
```

---

## Repository Structure

```text
gulliant-primitive/
├── README.md
├── LICENSE
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── lib.rs
│   ├── entrypoint.rs
│   ├── processor.rs
│   ├── instruction.rs
│   ├── state.rs
│   ├── error.rs
│   └── utils.rs
├── tests/
│   └── integration.rs
├── demo/
│   └── naive-transfer/
│       ├── README.md
│       ├── program/
│       │   └── src/lib.rs
│       └── client/
│           ├── Cargo.toml
│           └── src/main.rs
└── docs/
    ├── gulliant-deck.pdf
    └── archive/
        └── gulliant-primitive-deck.pdf
```

---

## License

Proprietary. A limited evaluation license is granted solely for the
Solana Frontier Hackathon.

See [LICENSE](./LICENSE) for full terms.
