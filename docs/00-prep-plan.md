# Prep plan: Sept 7 → Sept 28

You have 3 weeks of runway and then a 5-week build. The goal of the runway is
**not** to build the product early — check the rules on pre-existing code
first **(verify)**. The goal is that on day 1 nothing is a new skill.

Three things should be true by Sept 27:

1. You can write, test, and deploy a Solana program without looking anything up.
2. You have one idea you've committed to, and you know why the other candidates lost.
3. The riskiest technical assumption in that idea has already been proven in a throwaway spike.

Most teams fail the hackathon in week 1 of the build, burning it on toolchain
setup and idea churn. That week is what this plan buys back.

---

## Week 1 (Sept 7–13) — Toolchain fluency

Outcome: **a full ship cycle on something trivial.**

- [ ] Run `./scripts/setup-solana-dev.sh`. Get `anchor test` passing on the generated scaffold.
- [ ] Write a counter program from scratch — no tutorial open. Init a PDA, increment it, guard it so only the creator can increment.
- [ ] Deploy it to devnet. Note what the deploy cost you in SOL.
- [ ] Hit it from a TypeScript client. Then from a minimal React page with a wallet adapter.
- [ ] Break it deliberately: pass the wrong account and watch it fail. Read the error. Learn what Anchor errors look like before you're debugging one at 2am.

Do this end-to-end even though it's trivial. The value is entirely in having
done the boring parts once — the deploy, the IDL, the wallet connection, the
airdrop that silently rate-limits.

Work through [`docs/02-solana-ramp.md`](02-solana-ramp.md) alongside this.

## Week 2 (Sept 14–20) — Narrow to one idea

Outcome: **one idea, and a named person who wants it.**

- [ ] Read [`docs/01-idea-candidates.md`](01-idea-candidates.md). Score every candidate against the filter at the bottom.
- [ ] Cut to three. Write each as one sentence: who it's for, what it does, why it needs a blockchain.
- [ ] Take all three to actual people in that market. Not "would you use this" — ask what they do today and what it costs them.
- [ ] Kill two. Write down *why* each died; you'll get asked in judging.
- [ ] Spike the riskiest technical assumption of the survivor. Timebox it to two days. If it doesn't work, that's the week doing its job.

The spike matters more than it sounds. Every idea has one thing that either
works or doesn't — an extension that isn't as ready as the docs suggest, an
oracle that doesn't have your asset, a proof that takes 40 seconds in a
browser. Find it now, not in week 4.

## Week 3 (Sept 21–27) — Load the spring

Outcome: **a design you can start executing at 9am on the 28th.**

- [ ] Design the account model on paper. What's a PDA, what are the seeds, who pays rent, what's the size. This is the single highest-leverage hour of the whole hackathon.
- [ ] Draw the happy path as a sequence of transactions. Check each fits in one tx — see the size limit in the ramp doc.
- [ ] Decide your stack and don't revisit it. Anchor + Next.js + wallet-adapter is the boring correct answer unless you have a reason.
- [ ] Set up the repo: CI running `anchor build` and `anchor test` on push. Green from commit one.
- [ ] Settle the team question. Solo is viable; two is better; four strangers is worse than solo.
- [ ] Write the one-liner and the demo script *before* building. If you can't write a compelling 30-second pitch now, more code won't fix that.
- [ ] Read the official rules end to end **(verify)** — eligibility, team size, IP, what pre-work is allowed.

---

## During the build (Sept 28 – Nov 2)

Rough shape, adjust to the real deadline:

| Week | Focus |
|---|---|
| 1 | Core program. One instruction working end to end, deployed to devnet. |
| 2 | The rest of the program + tests. Feature-complete on-chain. |
| 3 | Frontend. Make the happy path beautiful; ignore every other path. |
| 4 | Mainnet deploy if you can afford it. Write the README. Start the video. |
| 5 | Video, submission, polish. **Reserve the entire last week.** |

The last week is not padding. Teams routinely build something good and submit
a bad video of it at 3am, and the video is what most judges actually watch.

## Standing risks

- **Devnet airdrops rate-limit hard.** Get SOL early, keep a funded keypair, know the faucet alternatives.
- **Program deploys cost real SOL** and cost it again on every redeploy. Budget for mainnet.
- **Scope creep disguised as ambition.** A narrow thing that works beats a broad thing that half-works, every single time.
- **"Why does this need Solana?"** If you can't answer in one sentence without saying "decentralization," pick a different idea.
