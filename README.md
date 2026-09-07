# Colosseum Fall 2026 — Hackathon Prep

Prep workspace for the Colosseum Solana hackathon, **Sept 28 – Nov 2, 2026**.

**Decided: spend-limited agent wallets.** See [`docs/04-agent-wallet-design.md`](docs/04-agent-wallet-design.md).

## Read in this order

| Doc | What it's for |
|---|---|
| [`docs/00-prep-plan.md`](docs/00-prep-plan.md) | Week-by-week plan for the 3 weeks before the clock starts |
| [`docs/01-idea-candidates.md`](docs/01-idea-candidates.md) | Candidate directions + the filter for choosing one |
| [`docs/02-solana-ramp.md`](docs/02-solana-ramp.md) | The Solana concepts that actually bite, in dependency order |
| [`docs/03-submission-strategy.md`](docs/03-submission-strategy.md) | What Colosseum rewards and how submissions are judged |
| [`docs/04-agent-wallet-design.md`](docs/04-agent-wallet-design.md) | **The chosen build.** Account model, policy semantics, threat model, test plan |
| [`docs/05-decision-log.md`](docs/05-decision-log.md) | What we decided, when, and why |
| [`docs/06-phase2-spike-findings.md`](docs/06-phase2-spike-findings.md) | **Spike results.** What the balance-delta approach does and doesn't catch |
| [`programs/agent-wallet/`](programs/agent-wallet/) | **The program.** Anchor implementation, 22 tests |
| [`app/`](app/) | **The frontend.** Demo surface, with a wire check against the program |

Runnable code: the program in [`programs/agent-wallet/`](programs/agent-wallet/),
the frontend in [`app/`](app/), and the spike in
[`spike/phase2-balance-delta/`](spike/phase2-balance-delta/).

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

## Conventions

Every file in this repo is divided by `##` section markers — `## Heading` in
markdown, `// ## Heading` or `# ## Heading` in code. Sections are the unit of
edit: to change something, locate its section and rewrite only that.

Get the map of any file without reading it:

```bash
grep -n '^#\{1,3\} \|^[/#]\+ ## ' README.md docs/*.md scripts/*.sh
```

Keep headings stable once written. Renaming one invalidates every reference to
it, including the cross-links between docs.
