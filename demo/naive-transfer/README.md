# Naive Transfer — Failure Proof

This is not Gulliant.

This is a minimal Solana program deployed on Devnet that demonstrates
a broken transfer model: player-bound state moves with the asset when
ownership changes.

---

## Program

Network: Solana Devnet  
Program ID: `FSTYFLyyyUAVGz5bak4waMr29gEawSwKeDgw5n1KBZhi`

```bash
solana program show FSTYFLyyyUAVGz5bak4waMr29gEawSwKeDgw5n1KBZhi --url devnet
```

---

## Failure Model

- `character_owner` changes to the buyer on transfer  
- `matchmaking_tier` remains attached to the asset  
- `permissions_level` remains attached to the asset  
- No on-chain mechanism prevents this  
- The buyer receives player-bound state that belonged to the seller  

---

## Reproduce

```bash
cargo run --quiet   --manifest-path demo/naive-transfer/client/Cargo.toml --   --url https://api.devnet.solana.com   --program-id FSTYFLyyyUAVGz5bak4waMr29gEawSwKeDgw5n1KBZhi   --payer ~/.config/solana/id.json   read --seller 4SknsQs8RNotCun6CcNpoXzG64XQg7uMZ3Wv7umuoEYM
```

---

## Expected Output

```text
====================
NAIVE TRANSFER STATE
====================
state_pda:         GhHuCr9orUHomjrPDRs1eUfgYE5bizZfAqWTZzh6xiZE
new_owner:         94hGJF4DDgvtZjBFVDYcKqxeXC9vaSpRchArJkbYSLmX
matchmaking_tier:  3  <- from previous player
permissions_level: veteran  <- from previous player

WARNING: This state belonged to the previous player
WARNING: It moved with the asset
```

The new owner holds player-bound state they did not earn.  
The previous player's state is no longer theirs to control.

---

## What This Violates

Player state is wallet-bound by definition.  
This program treats it as asset-bound by default.  
There is no enforcement boundary between the two.

---

## What Gulliant Enforces

Gulliant separates these at the program level:

- Player state is stored in a wallet-bound append-only log  
- The log does not transfer with the asset  
- Migration requires protocol authorization, snapshot validation, and single-use authorization  
- Invalid transitions are rejected on-chain  

See the root [`README.md`](../../README.md) for verification and test coverage.
