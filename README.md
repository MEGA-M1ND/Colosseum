# Colosseum Fall 2026 — Hackathon Prep

Prep workspace for the Colosseum Solana hackathon, **Sept 28 – Nov 2, 2026**.

## Read in this order

| Doc | What it's for |
|---|---|
| [`docs/00-prep-plan.md`](docs/00-prep-plan.md) | Week-by-week plan for the 3 weeks before the clock starts |
| [`docs/01-idea-candidates.md`](docs/01-idea-candidates.md) | Candidate directions + the filter for choosing one |
| [`docs/02-solana-ramp.md`](docs/02-solana-ramp.md) | The Solana concepts that actually bite, in dependency order |
| [`docs/03-submission-strategy.md`](docs/03-submission-strategy.md) | What Colosseum rewards and how submissions are judged |

## Setup

```bash
./scripts/setup-solana-dev.sh
```

Installs the Rust + Solana + Anchor toolchain, points you at devnet, and generates
a fresh Anchor workspace. Run it on your own machine — see the caveat below.

## Facts vs. assumptions

Everything in these docs marked **(verify)** came from secondhand search results,
not from Colosseum directly. `colosseum.com` is blocked from the environment these
docs were written in, so the official hackathon page, rules PDF, and blog were
unreadable. Confirm the dates, prize structure, and especially the **rules on
pre-existing code** before relying on any of it.

## Environment caveat

These docs were written in a sandbox with no access to `release.anza.xyz`,
`sh.rustup.rs`, or any Solana RPC endpoint. Nothing here has been compiled or run.
The setup script generates its scaffold with `anchor init` rather than hand-written
files, specifically so you get whatever the real toolchain produces instead of
something written blind against a guessed version.
