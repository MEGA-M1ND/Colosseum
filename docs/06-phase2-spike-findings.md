# Phase 2 spike — findings

Run on 2026-09-07, then extended twice the same day to fix both holes it found.
Code in [`../spike/phase2-balance-delta/`](../spike/phase2-balance-delta/).
Six tests, all passing, executed against the **real SPL Token program** under
`solana-program-test`.

## Verdict

**The mechanism works. Build it.** Both holes the spike found are now closed in
the spike itself, each with the exploit and its rejection kept as tests.

A raw balance delta turned out to be two things short of sufficient: it bounds
*movement* but not *authority*, and it bounds *one account* but not the vault.
Both fixes take the same shape as the original check — measure more state, never
parse the instruction — so the design's central claim survives intact.

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
| `over_budget_is_rejected_and_reverted` | 500 of a 100 cap → rejected `0x1`, vault back to 1000 |
| `approve_is_rejected` | zero-delta authority grant → rejected `0x2` |
| `set_authority_is_rejected` | close-authority grant → rejected `0x2` |
| `unmetered_mint_is_rejected` | drain of a second vault mint → rejected `0x3` |
| `untouched_sibling_does_not_block_a_valid_spend` | sibling in scope but unchanged → allowed |

## What was proven to break

Both holes from the design doc were suspected. Both are now demonstrated with a
passing test, which is a much stronger position for judging — you found them
yourself and can say what you did about it.

### `approve` slipped straight through — fixed

`approve_is_rejected` (was `approve_slips_past_the_delta_check`) — the agent
calls SPL Token `Approve` for the vault's entire 1000-token balance while under
a 100-token cap. It moves no tokens, so the delta is zero. **Before the fix the
guard allowed it**, and the delegate then drained the full balance in a separate
transaction that never touched the guard.

This was the sharpest hole. A balance delta measures value *movement*; `approve`
grants *authority*, which costs nothing until it is used.

**The fix — an authority fingerprint.** Snapshot every field on the token
account that grants standing power over it, and require it comes back unchanged:

```
owner            [32..64]
delegate         [72..108]    (COption tag + pubkey)
delegated_amount [121..129]
close_authority  [129..165]   (COption tag + pubkey)
```

`amount` is deliberately excluded — that one is allowed to change, and the delta
check is what bounds it.

The important property is that this does **not** forfeit the design. The guard
still never parses the instruction. It takes a second measurement of state
instead of interpreting intent, which is the same move as the balance check.
That means it catches instructions nobody has written yet, in exactly the way
the balance check does.

Verified by error code, not just by "the transaction failed":
the guard rejects with `custom program error: 0x2` (the fingerprint check),
the delegation is reverted rather than merely flagged, and the attacker's
follow-up drain then fails with token program error `0x4` — there is no
delegation to spend against.

`set_authority_is_rejected` covers the same class: handing an attacker close
authority over the vault account is also a zero-token operation, and the same
fingerprint catches it — also confirmed as `0x2`, so it is genuinely the guard
rejecting rather than the token program failing for an unrelated reason.

### An unmetered mint drained freely — fixed

`unmetered_mint_is_rejected` (was `unmetered_mint_drains_freely`) — the vault
holds two mints. The guard was pointed at mint A; the agent moved all of mint B.
Delta on A was zero, so **before the fix it was allowed**, past a cap of 100.

**The fix — put every vault holding the CPI can reach in scope.** Before the
call, scan the forwarded account list for anything that is a token account owned
by the vault authority, and snapshot each one. After the call, the metered
account is bounded by the cap and **every other vault holding must not shrink**.
It may grow; it may not fall.

Two properties make this work:

**The scan is complete.** A CPI can only touch accounts that were passed to it,
so everything the instruction could possibly drain is already in the forwarded
list. There is no hidden account for the agent to reach.

**The token program is not hardcoded.** The guard learns it from the metered
account's own `owner` field, then treats any forwarded account with that owner,
a token-account-sized buffer, and a token-owner matching the vault as a sibling.
The guard still knows nothing it was not told, which keeps the
works-against-unknown-programs claim honest.

Rejects with `custom program error: 0x3`, distinct from the authority check.

`untouched_sibling_does_not_block_a_valid_spend` is the counterweight. A guard
that refuses everything is not a guard — this passes a second vault holding
through the account list on a legitimate in-cap transfer and asserts it still
succeeds. Being in scope is not the same as being touched.

## Consequences for the design

1. **Authority fingerprint after every CPI — implemented and tested.** Carry it into the program as-is.
2. **Scope every vault-owned token account in the forwarded list — implemented and tested.** The metered one gets the cap; the rest must not shrink.
3. **Keep one mint per delegation anyway.** It is now defence in depth rather than the sole mitigation, and it keeps the accounting legible: a cap denominated in USDC means nothing applied to a basket.
4. **Meter native SOL lamports on the vault PDA, or explicitly scope SOL out.** Still not covered by the spike, and the same reasoning applies: an unmeasured field is a hole.

Error codes are distinct on purpose, so a test cannot pass for the wrong reason
and a user can tell what stopped them: `0x1` over cap, `0x2` authority altered,
`0x3` unmetered holding fell.

The generalisation worth carrying into the pitch: **a delta check is only as
good as the set of things it measures.** Every field an attacker can change that
you don't read is a hole, and so is every account. Both fixes here came from
widening what gets measured, not from learning to recognise an attack — which is
the property that makes the approach worth building on. Enumerate what you
measure, don't enumerate the attacks.

## Environment note

The spike had to be pinned to the Solana 2.x crate train — `spl-token` 8 is
built against 2.x component crates and will not link against `solana-program`
3.x or 4.x. Both were tried. See the spike README; this will cost you an hour
during the hackathon if you don't know it going in.

## Not covered

- Native SOL movement
- Freezing the vault account (denial of service rather than theft; needs the mint's freeze authority, not the vault's)
- CPI depth beyond one level
- Compute cost of the delta check under a realistic instruction
- Anchor integration (the spike is a native program; Anchor is ergonomics, not mechanism)
