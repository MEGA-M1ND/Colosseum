# Agent Wallet

**Give an AI agent a budget, not your keys.**

> Draft for the Colosseum submission. Fill the bracketed fields once devnet is
> live. Keep the Limitations section honest — judges find the gaps anyway, and
> volunteering them buys more credibility than hiding them costs.

- **Live demo:** [URL]
- **Program (devnet):** [PROGRAM_ID]
- **Demo video:** [URL]
- **Repo:** https://github.com/MEGA-M1ND/Colosseum

## The problem

Agents that transact need funds, and today that means handing them a private
key. One prompt injection — a hostile instruction buried in a webpage, an email,
a tool result — and the agent empties the wallet. There is no way to give an
agent spending power that is bounded by anything other than the agent's own
judgement.

## What we built

A Solana program that lets an owner grant an agent a **budget** instead of a
key. The owner funds a vault and issues a delegation to the agent's session key
carrying a policy:

- a cap per transaction
- a lifetime total
- a rolling-window rate limit
- an expiry
- a destination allowlist
- instant revocation

The agent signs with the session key. The program enforces the policy and signs
the token movement itself. **The owner's key never touches the agent**, and the
limits hold whether the agent is well-behaved, buggy, or fully compromised.

## Why this is not just a session key

Existing approaches scope by **capability**: this key may call these
instructions on these programs. That answers *what can it do* and leaves *how
much can it cost me* wide open. A whitelisted swap called in a loop drains the
wallet without ever leaving the allowlist.

This scopes by **economic damage**. And `agent_invoke` takes it further: it
executes an arbitrary instruction the program never parses, then bounds it by
measuring what left the vault.

That last property is the technical claim worth checking. The program does not
know what a swap is, or which field holds the amount. It measures the vault
before and after and refuses if too much moved — so it works against programs
that did not exist when it was written.

## Why Solana

The delegation lives in program-owned state and the vault's PDA signs its own
transfers, so enforcement happens at settlement rather than in a wrapper the
agent could route around. Fees and latency also matter here specifically: an
agent making frequent small payments is the exact workload where per-transaction
cost decides whether the product is viable.

## The security work

A balance check alone is not enough, and we found that by attacking it rather
than reasoning about it. Before writing the program we built a throwaway spike
and tried to rob our own vault. Three attacks worked:

| Now refused as | Attack | Why the first design missed it |
|---|---|---|
| `AuthorityAltered` (6012) | Approve a future spender | Grants an attacker the right to take funds later. Moves nothing now, so a spend limit sees a cost of zero. |
| `UnmeteredHoldingFell` (6013) | Move a different token | The budget covered one mint; the vault held two. Everything metered stayed still. |
| `VaultLamportsFell` (6014) | Take the SOL, not the tokens | Never touches the token program — which is exactly why every token-shaped check passed it. |

Each is closed, and each is kept as a passing test alongside the exploit that
found it. See [`docs/06-phase2-spike-findings.md`](docs/06-phase2-spike-findings.md)
and [`spike/phase2-balance-delta/`](spike/phase2-balance-delta/).

All three shared a root cause, and all three fixes were the same move: **widen
what gets measured**, never teach the guard to recognise an attack. A guard that
had learned to spot token attacks would have been blind to the third one,
because it is not one.

## Architecture

```
owner wallet ──grants policy──▶ Delegation (PDA)
                                    │ per-tx / total / window / expiry / revoke
                                    ▼
agent session key ──signs──▶ agent_transfer   ──┐
                             agent_invoke     ──┤─▶ Vault (PDA) ──▶ SPL Token
                                                │        │
                                                │   measured before + after:
                                                │   balance, authority fields,
                                                │   sibling holdings, lamports
                                                └── reject ⇒ whole tx reverts
```

Two accounts. The **Vault** holds funds and a pause switch. The **Delegation**
holds one agent's policy and its spend counters — so two agents get independent
budgets, and revoking one leaves the other untouched.

## Testing

- **23 tests** on the program, against the real SPL Token program
- **7 tests** on the security spike, holding the three attacks and their refusals
- Every rejection asserts its **specific error code**, not merely that something failed — so no test can pass because an instruction was malformed and the guard never ran
- The frontend's wire format is verified **byte-for-byte** against reference vectors emitted by the program's own test suite: 9 instructions, 20 error codes, both account layouts

```bash
cargo test -p agent-wallet            # 23
cd spike/phase2-balance-delta && cargo test   # 7
cd app && npm run check:wire          # client vs program
```

## Limitations

Stated plainly rather than discovered:

- **[UPDATE BEFORE SUBMITTING]** — remove any of these that no longer apply
- The delegation covers one SPL mint. Multi-mint budgets are not supported, deliberately: it is what keeps the metering sound.
- SOL is not spendable by the agent at all. A lamport floor rejects any instruction that reduces the vault's SOL, which also means the vault cannot pay rent inside an agent call.
- The rate limit is a tumbling window, not a sliding one. It resets wholesale on expiry, so an agent can spend the full window at the end of one and again at the start of the next. Set the window to half your real tolerance. A sliding window needs a ring buffer of timestamped spends for a marginal gain.
- `agent_invoke` is opt-in and its program allowlist is empty by default.
- The program bounds the loss; it does not judge the trade. An agent making bad-but-permitted decisions is not something this prevents.

## Prior work

**[UPDATE — this is the honest disclosure. Adjust to the facts and the rules.]**

The on-chain program, the security spike, and the frontend scaffold were written
between 7 and 17 September 2026, before the hackathon opened. The commit history
is public and dated. Work completed during the hackathon window: [LIST].

## Team

**[NAME]** — [one or two lines on why you, specifically, for this problem]
