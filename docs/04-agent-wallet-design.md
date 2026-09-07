# Agent wallet — design

## Summary

A Solana program that lets a human grant an AI agent **spending authority
without key custody**. The owner funds a vault PDA and issues a delegation to an
agent's session key carrying a policy: per-transaction cap, lifetime cap, a
rolling-window rate limit, an expiry, a destination allowlist, and a revoke
switch. The agent signs with the session key; the program enforces the policy
and signs the actual token movement as the vault PDA.

The owner's keys never touch the agent. Revocation is one transaction and takes
effect immediately, because every spend re-reads the delegation account.

## The sharp answer on "session keys already exist"

You will get this question in judging. The honest differentiator is not the
session key — it's **what the policy measures**.

Existing approaches scope by *capability*: this key may call these instructions
on these programs. That answers "what can it do" and leaves "how much can it
cost me" completely open. A whitelisted swap called in a loop drains the wallet
without ever leaving the allowlist.

This design scopes by *economic damage*: the vault accounts for value leaving
it, per transaction, per window, and in total, and refuses when the number is
exceeded. Phase 2 extends the same idea to arbitrary CPI by measuring the
balance delta across the call rather than trying to parse what the call does.

That framing is the wedge, and it's what the demo should show.

## Threat model

What the program defends against:

| Threat | Defense |
|---|---|
| Agent is prompt-injected into draining the wallet | Per-tx and total caps; balance-delta check in phase 2 |
| Agent goes into a loop / runaway | Rolling-window rate limit |
| Session key leaks | Expiry, plus owner revocation in one tx |
| Agent sends to an attacker address | Destination allowlist |
| Compromised agent calls a malicious program | Program allowlist (phase 2) |
| Owner needs to stop everything at once | Vault-level pause flag |

Explicitly **not** defended: an agent making bad-but-permitted decisions. The
program bounds the loss, it doesn't judge the trade. Say this plainly in the
README — it reads as clear thinking, not as a gap.

## Account model

Two accounts. Both PDAs, both owned by the program.

**Vault** — `seeds = [b"vault", owner]`

```rust
pub struct Vault {
    pub owner: Pubkey,      // only key that may create/revoke delegations
    pub paused: bool,       // global kill switch
    pub bump: u8,
}
```

Funds live in an **ATA owned by the vault PDA**, so the vault custodies real
SPL tokens and the program signs transfers out of it with `invoke_signed`.

**Delegation** — `seeds = [b"delegation", vault, agent]`

One active delegation per (vault, agent, mint) pair. Deriving from the agent
pubkey means the client always knows the address without an index.

```rust
#[derive(InitSpace)]
pub struct Delegation {
    pub vault: Pubkey,
    pub agent: Pubkey,              // the session key
    pub mint: Pubkey,               // delegation is per-token

    pub per_tx_limit: u64,
    pub total_limit: u64,
    pub total_spent: u64,

    pub window_duration: i64,       // seconds; 0 disables the window
    pub window_limit: u64,
    pub window_spent: u64,
    pub window_started_at: i64,

    pub expires_at: i64,            // unix seconds
    pub revoked: bool,

    #[max_len(8)]
    pub allowed_destinations: Vec<Pubkey>,  // empty = any destination
    #[max_len(8)]
    pub allowed_programs: Vec<Pubkey>,      // phase 2

    pub bump: u8,
}
```

Roughly 682 bytes with the discriminator. Cap both vectors at 8 — unbounded
vectors are a rent and transaction-size problem, and 8 is plenty for a demo.

Design note worth defending in judging: **spend counters live on the delegation,
not the vault**. Two agents on one vault have independent budgets and can't
starve each other, and revoking one leaves the other untouched.

## Instructions

Phase 1 — build this first, it is a complete demo on its own.

| Instruction | Signer | Does |
|---|---|---|
| `initialize_vault` | owner | Creates the vault PDA |
| `deposit` | owner | Moves tokens into the vault ATA |
| `create_delegation` | owner | Issues a policy to an agent key |
| `update_delegation` | owner | Adjusts limits on a live delegation |
| `revoke_delegation` | owner | Sets `revoked`; effective immediately |
| `set_paused` | owner | Vault-wide stop |
| `agent_transfer` | **agent** | Spends from the vault, subject to policy |
| `withdraw` | owner | Owner pulls funds back out |

`agent_transfer` is the whole product. Everything else is scaffolding around it.

## Policy evaluation

The order matters — cheapest and most-decisive checks first, and every
arithmetic operation is checked.

```
1. vault.paused == false
2. delegation.revoked == false
3. now < delegation.expires_at
4. delegation.agent == agent.key()        (agent is a Signer)
5. delegation.vault == vault.key()
6. delegation.mint  == mint.key()
7. amount > 0 && amount <= per_tx_limit
8. total_spent.checked_add(amount) <= total_limit
9. rolling window:
      if window_duration > 0:
          if now >= window_started_at + window_duration:
              window_started_at = now
              window_spent      = 0
          window_spent.checked_add(amount) <= window_limit
10. allowed_destinations is empty OR contains destination owner
11. CPI: token transfer from vault ATA -> destination, signed by vault PDA
12. commit total_spent and window_spent
```

Two things to get right:

**Update state after the CPI succeeds, not before** — or use Anchor's ordering
carefully. A failed CPI reverts the whole transaction anyway, but the habit
matters once phase 2 introduces calls that can partially succeed.

**The window is a tumbling window, not a sliding one.** It resets wholesale when
it expires rather than tracking a true rolling sum. This is deliberate: a
sliding window needs a ring buffer of timestamped spends, which costs account
space and compute for very little added safety. Know this trade-off — a sharp
judge may ask, and "we chose tumbling because a sliding window costs N slots of
state for a marginal tightening" is a good answer. "We didn't think about it" is
not.

## Phase 2 — arbitrary CPI with balance-delta enforcement

This is the differentiator. Build it in week 3, only once phase 1 is solid.

`agent_invoke` lets the agent hand the program an arbitrary instruction to
execute as the vault:

```
1. target program ID ∈ delegation.allowed_programs
2. record vault ATA balance BEFORE
3. invoke_signed(instruction, remaining_accounts, vault_seeds)
4. reload the token account; record balance AFTER
5. spent = before.checked_sub(after)
6. run spent through the same policy checks as agent_transfer
7. commit counters
```

Why this is interesting: the program never parses the target instruction. It
doesn't need to know what a swap is, or which field is the amount. It measures
what left the vault. That generalizes to programs that didn't exist when you
wrote yours, which is exactly the property an agent seatbelt needs.

Known gaps to state honestly rather than paper over:
- Only accounts you check are measured. An instruction that moves a *different*
  mint out of a different vault ATA isn't caught unless you check that ATA too.
  Constrain the delegation to a single mint and check that one — and say so.
- Approvals aren't spends. A call that sets a token delegate costs nothing now
  and everything later. Either forbid the SPL `approve` instruction or note the
  limitation. **Do not skip this one; it's the sharpest hole and a good judge
  will find it.**
- CPI depth is 4. Deeply nested targets will fail.

## Security checklist

Cross-referenced with the footgun list in [`02-solana-ramp.md`](02-solana-ramp.md#7-security-footguns):

- [ ] `agent` is `Signer<'info>` — not just a passed pubkey
- [ ] `owner` is `Signer` on every owner-only instruction
- [ ] `delegation.vault == vault.key()` and `delegation.agent == agent.key()` — via `has_one`
- [ ] Vault ATA verified as the canonical ATA of the vault PDA for `mint`
- [ ] Canonical bumps only — use Anchor's `bump` in seed constraints, never a caller-supplied bump
- [ ] `checked_add` / `checked_sub` on every counter; no bare `+`
- [ ] Phase 2: target program ID checked against the allowlist **before** `invoke_signed`
- [ ] Phase 2: token account `reload()`ed after the CPI — a stale deserialization reads the pre-call balance and defeats the entire check
- [ ] `revoked` and `expires_at` re-read on every spend, never cached client-side

## Test plan

The tests are a judging asset — say in the README that the seatbelt is tested,
and these are the cases.

Happy path:
- [ ] Owner creates vault, deposits, delegates; agent spends under all limits

Each limit rejects:
- [ ] Spend over `per_tx_limit`
- [ ] Cumulative spend crossing `total_limit`
- [ ] Second spend inside the window crossing `window_limit`
- [ ] Spend after the window rolls over — **succeeds**, counter reset
- [ ] Spend after `expires_at`
- [ ] Spend after `revoke_delegation`
- [ ] Spend while vault is paused
- [ ] Spend to a destination not on the allowlist

Authorization:
- [ ] A different key signing with a valid delegation account is rejected
- [ ] A non-owner attempting `create_delegation` / `revoke_delegation` is rejected
- [ ] Agent A's delegation cannot spend against agent B's budget

Arithmetic:
- [ ] `u64::MAX` amount doesn't overflow the counters

## Demo script

Ninety seconds, in this order. Build toward the rejection — that's the shot.

1. Owner funds a vault, grants an agent 100 USDC with a 10 USDC per-tx cap.
2. Agent does its actual job, autonomously, a few small spends. Show it working.
3. Agent gets prompt-injected — a hostile string in a tool result telling it to send everything to an attacker address.
4. Agent tries. **The transaction fails on-chain.** Show the explorer error.
5. Owner hits revoke. Agent's next legitimate call fails too.

Step 3 is what makes it land. The injection makes the threat concrete instead of
hypothetical, and it's the difference between "a spending limit" and "the thing
that saves you." Script the injection so it's reproducible on camera.

## Open questions

- **SOL as well as SPL tokens?** Native SOL needs different handling than the token CPI path. Recommendation: SPL only, note it as scoped.
- **Multiple mints per delegation?** Recommendation: no. One mint per delegation keeps the balance-delta check sound in phase 2.
- **Who pays transaction fees, agent or owner?** The agent needs some SOL for fees. Simplest is to fund the session key with a small amount and mention it.
- **Should the delegation account close on revoke?** Closing refunds rent but loses the audit trail. Recommendation: keep it, flag `revoked`, and expose the spend history — an auditable record of what the agent did is itself a selling point.
