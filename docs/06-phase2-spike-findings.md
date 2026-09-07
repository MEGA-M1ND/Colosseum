# Phase 2 spike — findings

Run on 2026-09-07. Code in [`../spike/phase2-balance-delta/`](../spike/phase2-balance-delta/).
Four tests, all passing, executed against the **real SPL Token program** under
`solana-program-test`.

## Verdict

**The mechanism works. Build it.** But it is a *cap on token movement*, not a
cap on authority, and the difference is exploitable in two specific ways that
the spike demonstrates rather than speculates about.

## What was proven to work

**Balance reads after a CPI are fresh.** This was the load-bearing uncertainty.
If the guard re-read stale pre-CPI data the whole approach would be silently
unsound — it would report a delta of zero for every call and wave everything
through. It does not. The runtime writes CPI results back into the shared
account buffer, and a fresh `try_borrow_data()` after `invoke_signed` sees the
post-call balance.

**Measuring after the fact is safe.** The value has already moved by the time
the guard checks. Returning `Err` reverts the CPI atomically along with the rest
of the transaction. `over_budget_is_rejected_and_reverted` asserts the vault is
back at its full starting balance after the rejection, and the runtime log shows
`custom program error: 0x1` — the guard's reject path.

**The guard never parses the instruction.** It forwards opaque bytes to a
program it knows nothing about and judges only the result. That is the property
worth pitching: it works against programs that did not exist when it was
written.

| Test | Result |
|---|---|
| `within_budget_is_allowed` | 50 of a 100 cap → allowed, vault 1000 → 950 |
| `over_budget_is_rejected_and_reverted` | 500 of a 100 cap → rejected, vault back to 1000 |

## What was proven to break

Both holes from the design doc were suspected. Both are now demonstrated with a
passing test, which is a much stronger position for judging — you found them
yourself and can say what you did about it.

### `approve` slips straight through

`approve_slips_past_the_delta_check` — the agent calls SPL Token `Approve` for
the vault's entire 1000-token balance while under a 100-token cap. It moves no
tokens, so the delta is zero and **the guard allows it**. The delegate then
transfers the full balance in a separate transaction that never touches the
guard. Vault ends at 0, past a cap of 100.

This is the sharpest hole. A balance delta measures value *movement*; `approve`
grants *authority*, which costs nothing until it is used.

**Fix:** the delegation must reject instructions that grant standing authority.
Because the guard deliberately doesn't parse instructions, the check has to be
structural — after the CPI, assert the metered account's `delegate` field is
still unset and `delegated_amount` is still zero. Same shape as the balance
check: measure state, don't interpret intent. Also assert the account's `owner`
and `close_authority` are unchanged, since `SetAuthority` is the same class of
attack.

### An unmetered mint drains freely

`unmetered_mint_drains_freely` — the vault holds two mints. The guard meters
mint A; the agent moves all of mint B. Delta on A is zero, so it is allowed.

**Fix:** this is what the "one mint per delegation" decision already buys, and
the test is the evidence for it. Worth also asserting the vault holds no other
token accounts, or metering every account passed in `remaining_accounts` that
the vault owns.

## Consequences for the design

Three changes to [`04-agent-wallet-design.md`](04-agent-wallet-design.md) phase 2:

1. After the CPI, assert on the metered account: `delegate == None`, `delegated_amount == 0`, `owner` unchanged, `close_authority == None`. Reject otherwise.
2. Keep one mint per delegation. It is now a tested requirement, not a preference.
3. Meter native SOL lamports on the vault PDA as well, or explicitly scope SOL out. The spike did not cover it.

The generalisation worth carrying into the pitch: **a delta check is only as
good as the set of things it measures.** Every field an attacker can change that
you don't read is a hole. Enumerate the fields, don't enumerate the attacks.

## Environment note

The spike had to be pinned to the Solana 2.x crate train — `spl-token` 8 is
built against 2.x component crates and will not link against `solana-program`
3.x or 4.x. Both were tried. See the spike README; this will cost you an hour
during the hackathon if you don't know it going in.

## Not covered

- Native SOL movement
- CPI depth beyond one level
- Compute cost of the delta check under a realistic instruction
- Anchor integration (the spike is a native program; Anchor is ergonomics, not mechanism)
