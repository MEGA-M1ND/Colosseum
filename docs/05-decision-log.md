# Decision log

Append-only. Each entry: what was decided, why, and what would reverse it.
When a judge asks "why did you build it this way," this is the answer sheet.

## 2026-09-07 — Build spend-limited agent wallets

Chosen over confidential payroll and vertical stablecoin rails.

**Why:** best ratio of judge interest to build risk at current Solana depth. The
demo is self-explanatory in one shot. Agent autonomy is the live theme and the
safety layer is genuinely unbuilt rather than merely unshipped.

**Reverses if:** week 2 user conversations find nobody actually running agents
with funds at stake, or a well-funded team ships the same seatbelt before the
hackathon opens.

## 2026-09-07 — Policy scopes economic damage, not capability

The differentiator against existing session keys. Allowlists answer "what can it
call"; this answers "how much can it cost me." Phase 2 extends it to arbitrary
CPI by measuring balance deltas instead of parsing instructions.

**Reverses if:** the balance-delta approach turns out unsound in the phase 2
spike. Fall back to capability scoping and lead with the rate limiter instead.

## 2026-09-07 — Tumbling window, not sliding

A true sliding window needs a ring buffer of timestamped spends: more account
space, more compute, marginal safety gain. Tumbling resets wholesale on expiry.

**Reverses if:** a reviewer demonstrates a practical attack that straddles the
reset boundary — spend the full window at the end of one and again at the start
of the next, i.e. 2x the limit in a short span. Known and accepted; mitigate by
telling users to set the window at half their real tolerance.

## 2026-09-07 — Spend counters on the delegation, not the vault

Independent budgets per agent. Two agents can't starve each other, and revoking
one doesn't disturb the other.

**Reverses if:** a vault-wide cap across all agents turns out to be the thing
users actually ask for. Additive, not a rewrite — a vault-level counter can be
checked alongside.

## 2026-09-07 — One mint per delegation

Keeps the phase 2 balance-delta check sound: one token account to measure.
Multi-mint would need a delta check per mint and opens a hole where the agent
drains an unmeasured one.

**Reverses if:** never, for the hackathon. This is a correctness constraint, not
a scoping one.

## 2026-09-07 — Program code deferred until hackathon rules are confirmed

The design is written; the program is not. Colosseum's rules on pre-existing
code could not be read from the build environment. Writing the program during
the runway risks disqualification for an unknown gain.

**Reverses if:** the rules permit disclosed prior work, or the hackathon opens.
Either way the design above is the spec and the program follows from it directly.

## 2026-09-07 — Phase 2 balance-delta approach validated, with two required fixes

Spike run against the real SPL Token program: post-CPI balance reads are fresh,
and rejecting after the fact reverts atomically. The mechanism holds.

Two demonstrated holes, both now covered by passing tests: `approve` grants
authority at zero delta, and an unmetered second mint drains freely.

**Consequence:** phase 2 must also assert `delegate`, `delegated_amount`,
`owner` and `close_authority` are unchanged after the CPI. One mint per
delegation is promoted from preference to tested requirement.

**Reverses if:** nothing found so far. The approach survived the spike.

## 2026-09-07 — Close the approve hole with an authority fingerprint

Rather than blocklisting `approve`, snapshot every field on the token account
that grants standing power — owner, delegate, delegated_amount, close_authority
— and require it unchanged after the CPI. `amount` is excluded; the delta check
bounds that.

**Why this shape:** blocklisting instructions would forfeit the design's whole
claim, which is that the guard works against programs it has never seen. A
second state measurement keeps that property. It caught `set_authority` for
free, which is the evidence the generalisation is real.

**Reverses if:** a legitimate integration needs the agent to set a delegate.
Then the allowance becomes explicit policy on the delegation rather than a
blanket prohibition.

## 2026-09-07 — Close the unmetered-mint hole by scoping the whole account list

Before the CPI, snapshot every token account in the forwarded list that the
vault owns. After it, the metered account is bounded by the cap and no other
vault holding may shrink.

**Why this is sound:** a CPI can only touch accounts passed to it, so scanning
the forwarded list covers everything the instruction could reach. The token
program is learned from the metered account's `owner` rather than hardcoded, so
the guard still knows nothing it was not told.

**Consequence for an earlier entry:** one-mint-per-delegation is no longer the
only thing standing between an agent and the vault's other holdings. Keep it as
defence in depth and for legible accounting — a cap denominated in one token
means nothing applied to a basket.

**Reverses if:** the scan proves too expensive in compute on a realistic
instruction with many accounts. Measure before assuming; the loop is over
accounts already in the list.

## 2026-09-07 — Close the native SOL hole with a lamport floor

Snapshot lamports on the vault PDA, the metered token account and every sibling;
reject if any falls. Confirmed as a live exploit first: a System program
transfer signed by the vault PDA drained the vault's SOL while passing every
token-shaped check, because the attack never went near the token program.

**Why a floor rather than a cap:** the delegation grants a token budget, not a
SOL budget. Agent transaction fees are paid by the session key, not the vault,
so ordinary operation never touches vault lamports.

**Known cost:** an instruction where the vault legitimately pays rent is now
rejected. Accepted — the demo path does not need it.

**Reverses if:** a flow needs the vault to fund account creation. Then SOL gets
an explicit allowance on the delegation, shaped like the token cap.

## 2026-09-07 — Write the program; pin Anchor 0.31 rather than 1.2

The program is written and tested: nine instructions, 22 passing tests against
the real SPL Token program.

**Anchor 0.31.1, not 1.2.** Anchor 1.2 depends on `solana-invoke` 0.5, whose
non-SBF path is a bare `unimplemented!()` with no stub fallback and no feature
flag. Any CPI — including `init`, which creates accounts via the system program
— panics under `solana-program-test`'s native executor. Choosing 1.2 would have
traded the entire test suite for a version number.

The program was compiled clean against 1.2 first, so the migration surface is
known and small: `Context` lost its extra lifetimes, and `CpiContext::new` takes
the program id rather than its `AccountInfo`. Both are recorded in the program
README.

**Reverses if:** the SBF toolchain becomes reachable, or you decide current-Anchor
matters more than native tests. Migration is two mechanical edits.

**Not verified anywhere yet:** `anchor build`, the SBF binary, the IDL, and
deployment. `cargo-build-sbf` needs `release.anza.xyz`, which is blocked here.
Run `anchor build && anchor test` locally before trusting the deploy path.
