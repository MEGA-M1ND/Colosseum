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
