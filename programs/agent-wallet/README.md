# agent-wallet

Give an AI agent a budget, not your keys.

## Run the tests

```bash
cargo test -p agent-wallet     # 22 tests
```

No validator and no SBF toolchain needed: `solana-program-test` runs the Anchor
program and the real SPL Token program natively via `processor!`.

## What is verified, and what is not

**Verified here:** the program compiles, and all 22 tests pass against the real
SPL Token program — every policy limit, the authorization constraints, and all
four phase 2 guard rejections.

**Not verified here:** `anchor build`, the SBF binary, the generated IDL, and
deployment. `cargo-build-sbf` downloads platform tools from `release.anza.xyz`,
which was unreachable from the environment this was written in. Run
`anchor build && anchor test` locally before trusting the deploy path.

## Layout

| File | Contents |
|---|---|
| `src/lib.rs` | The nine instructions and their account contexts |
| `src/state.rs` | `Vault`, `Delegation`, policy evaluation |
| `src/guard.rs` | Phase 2 measurement: fingerprint, sibling scope, lamport floor |
| `src/errors.rs` | One code per failure mode |
| `tests/policy.rs` | Every limit, proven to reject (9) |
| `tests/auth.rs` | Who may do what (5) |
| `tests/invoke.rs` | Phase 2 attacks and their rejections (6) |
| `tests/smoke.rs` | End-to-end happy path (1) |

## Anchor version

Pinned to **Anchor 0.31.1**, not 1.2. This is a deliberate trade.

Anchor 1.2 depends on `solana-invoke` 0.5, whose non-SBF path is a bare
`unimplemented!()` with no syscall-stub fallback and no feature flag to restore
one. Any program that makes a CPI — which here includes `init`, since account
creation is a system-program CPI — panics the moment it runs under
`solana-program-test`'s native executor. That would have cost the entire test
suite: the program would compile and nothing else could be checked.

0.31.1 routes through `program_stubs`, so the tests above actually run.

**Migrating to 1.x later is small.** The only two API changes this program hits:

```rust
// Context lost its extra lifetimes
Context<'_, '_, 'info, 'info, AgentInvoke<'info>>   // 0.31
Context<'info, AgentInvoke<'info>>                  // 1.x

// CpiContext takes the program id, not its AccountInfo
CpiContext::new(ctx.accounts.token_program.to_account_info(), ..)  // 0.31
CpiContext::new(ctx.accounts.token_program.key(), ..)              // 1.x
```

Both were compiled successfully against 1.2 before the downgrade. If you move to
1.x, expect to lose native `cargo test` and run `anchor test` instead.

## Instructions

| Instruction | Signer | Does |
|---|---|---|
| `initialize_vault` | owner | Creates the vault PDA |
| `set_paused` | owner | Vault-wide stop |
| `create_delegation` | owner | Issues a policy to an agent key |
| `update_delegation` | owner | Adjusts limits; never resets spend counters |
| `revoke_delegation` | owner | Immediate; every spend re-reads the flag |
| `deposit` | owner | Funds the vault |
| `withdraw` | owner | Owner escape hatch, outside any policy |
| `agent_transfer` | **agent** | The product: a bounded spend |
| `agent_invoke` | **agent** | Arbitrary CPI, bounded by measurement |

## Errors

| Code | Meaning |
|---|---|
| 6000-6010 | Policy: paused, revoked, expired, over a limit, allowlist |
| `AuthorityAltered` | The CPI granted standing authority (`approve`, `set_authority`) |
| `UnmeteredHoldingFell` | Another vault holding decreased |
| `VaultLamportsFell` | Native SOL left the vault |

The last three are the phase 2 guard. Each was a working exploit in
`spike/phase2-balance-delta/` before it was a rejection — see
[`../../docs/06-phase2-spike-findings.md`](../../docs/06-phase2-spike-findings.md).
