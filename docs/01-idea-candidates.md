# Idea candidates

Frontier (spring 2026) ran with **no tracks and no bounties** — judged purely on
overall product impact **(verify this holds for fall)**. That changes the
strategy: you're not fitting a category, you're competing against ~2,800
submissions on whether the thing is good. Colosseum invests in winners, so
you're being read as a pre-seed company, not a weekend project.

Calibrated to your position: you can build, Solana depth is new, five weeks,
starting from nothing.

## What that position rules out

**Don't build infrastructure.** A new DEX, a bridge, a rollup, an oracle
network. You'd be competing against teams who've lived in it for years, and
infra demos badly — there's nothing to show.

**Don't build anything needing a cold-start network.** Two-sided marketplaces,
social graphs, anything whose value is "once there are 10,000 users." You have
five weeks and a demo, not users.

**Don't pick something whose hard part is off-chain.** If the interesting
engineering is an ML model or a scraper, the Solana part reads as decoration
and judges notice immediately.

The shape that fits: **a narrow product, on a primitive that shipped recently
and that nobody has built product on yet, with a demo where something visibly
happens.**

---

## Candidates

### 1. Spend-limited agent wallets
**Strongest fit for your position.**

A delegation program: you grant an AI agent a budget from your wallet with
per-transaction caps, a rate limit, an allowlist of programs it may call, and
instant revocation. The agent holds a session key, never your keys.

- **Why now** — agents transacting autonomously is the live theme (Colosseum ran a dedicated AI Agent hackathon in Feb 2026), and "how do I not give a bot my keys" is genuinely unsolved rather than merely unbuilt.
- **The wedge** — everyone is shipping agents that *can* transact. Almost nobody is shipping the seatbelt.
- **Scope** — a PDA-based delegation program plus a small client. This is honestly tractable in five weeks.
- **Demo moment** — agent buys within budget, works. Agent tries to drain the wallet, program rejects it on-chain. That's a 20-second clip that explains itself.
- **Risk** — "session keys" exist in various forms; you need a sharp answer on what's different. Lean on the policy engine being expressive and on-chain-enforced.
- **Solana depth** — medium. PDAs, CPI, signer checks. All learnable in week 1.

### 2. Confidential payroll on Token-2022

Token Extensions shipped confidential transfers, and there is very little
product built on top. Payroll is the obvious use: crypto-native companies pay
contractors on-chain, and nobody wants every salary public forever.

- **Why now** — the primitive is live and under-exploited. Judges reward building on new rails.
- **The wedge** — "your cap table is public" is a real objection to on-chain payroll that this actually removes.
- **Demo moment** — show the block explorer with amounts hidden, then the recipient decrypting theirs. Strong visual.
- **Risk** — the highest technical risk here. Client-side proof generation, auditor keys, and rough tooling. **Spike this in week 2 before committing.**
- **Solana depth** — high. Only pick this if week 1 goes very smoothly.

### 3. Stablecoin rails for one specific vertical

Not "a payments app" — cross-border invoicing for a market you personally know.
Freelancers billing US clients, small importers, a specific remittance corridor.

- **Why now** — stablecoin settlement is Solana's most defensible real-world story, and the fee/latency argument makes itself.
- **The wedge** — entirely in the vertical. The one who wins is the one whose founder obviously knows the market. Generic checkout loses.
- **Scope** — low program complexity, most of the work is product. Good match for your Solana depth.
- **Demo moment** — real invoice, real settlement, seconds, cents. Show the fee next to a wire transfer fee.
- **Risk** — judges have seen many payment apps. Without a specific market you can speak about credibly, this is a middle-of-the-pack submission.
- **Solana depth** — low. Mostly SPL token transfers and Solana Pay.

### 4. Transfer-hook compliance rails

Transfer hooks let a program execute on every token transfer — allowlists,
royalties, per-transfer logic. Real demand from RWA and gated-token issuers.

- **Why now** — the primitive is under-built on.
- **Risk** — compliance demos badly. It's a screen of rules being enforced, which is dull, and the buyer is an institution you can't reach in five weeks.
- **Verdict** — technically interesting, weak as a hackathon submission. Listed so you can rule it out deliberately.

### 5. A Blinks-native consumer product

Blinks/Actions put a Solana transaction inline in a social feed. Low program
complexity, distribution built in, demos well.

- **Upside** — you could genuinely ship this, and people can use it from a link during judging.
- **Risk** — reads as a toy. Needs a real reason for the transaction to exist, or judges file it under "cute."
- **Verdict** — viable if paired with actual substance underneath.

### 6. Verifiable off-chain compute settlement
Listed as a **trap**. Perennially attractive, perennially too large. Teams pick
it, spend four weeks on proof plumbing, and submit an architecture diagram.
Don't.

---

## The filter

Score each surviving candidate 1–5. Anything under 4 on the first two is dead
regardless of the total.

| Test | The question |
|---|---|
| **Demo** | Can someone who knows nothing watch 90 seconds and understand what happened? |
| **Why Solana** | One sentence, no hand-waving about decentralization. Would a Postgres table do this? |
| **Reach** | Can *you*, at your current depth, have this working in five weeks — not designed, working? |
| **A real person** | Can you name someone who wants it? Not a persona. A person. |
| **Wedge** | If a stronger team builds the same thing, what do you still have? |

Weight **Demo** and **Why Solana** hardest. They're what actually kills
submissions, and they're the two things founders reliably talk themselves out of
being honest about.

## My read

Start on **#1 (spend-limited agent wallets)**. It has the best ratio of judge
interest to build risk for someone at your depth, and the demo writes itself.

Take **#3** seriously if there's a market you genuinely know from the inside —
founder-market fit beats novelty in a competition that ends in an investment
decision, and it's the lower-risk build.

Treat **#2** as the upside play: spike it in week 2, commit only if the
confidential-transfer tooling cooperates.
