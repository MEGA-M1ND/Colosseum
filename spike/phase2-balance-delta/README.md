# Phase 2 spike — balance-delta enforcement

## What this is

A **throwaway spike**, not product code. It exists to answer one question before
the hackathon: can a guard program let an agent invoke an arbitrary instruction
as the vault, without parsing that instruction, and still bound how much value
leaves?

Findings are written up in [`../../docs/06-phase2-spike-findings.md`](../../docs/06-phase2-spike-findings.md).

## Run it

```bash
cd spike/phase2-balance-delta
cargo test                       # 6 tests
cargo test -- --nocapture        # with program logs
```

No validator and no SBF toolchain needed. `solana-program-test` runs both the
guard and the real SPL Token program natively in-process via `processor!`.

## Files

- `src/lib.rs` — the guard: snapshot balance, authority fields and sibling vault holdings; `invoke_signed` blind; re-measure; reject
- `tests/delta.rs` — six cases: two allowed, three attacks rejected, one over-rejection guard

## Version pinning matters

Pinned to the Solana **2.x** train:

```toml
solana-program = "2"      spl-token = "8"
solana-program-test = "2" solana-sdk = "2"
```

This is not arbitrary. `spl-token` 8 is built against the 2.x component crates
(`solana-account-info` 2.3, `solana-program-pack` 2.2). Mixing it with
`solana-program` 3.x or 4.x gives you two incompatible copies of the same types
in one dependency graph, and `processor!(spl_token::processor::Processor::process)`
will not compile. Both the 3.x and 4.x combinations were tried and failed.
