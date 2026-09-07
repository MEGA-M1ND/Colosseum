# Submission strategy

## Understand what this competition actually is

Colosseum deploys capital into winners — the spring pool was $2.75M **including
pre-seed investment**, with $30K to the Grand Champion and $10K each to the next
20 **(verify for fall)**. That reframes everything: judges are reading
submissions as **companies**, not as projects.

The practical consequence is that the polished demo of a narrow, obviously-real
product beats the ambitious half-working platform. "Would I put money into this
team" is a different question from "is this impressive code," and it's the one
being asked.

With ~2,800 submissions in the spring, assume your first impression is measured
in seconds.

## The demo video is the submission

Most judges watch the video and never run your code. Optimize accordingly.

- **Keep it under 3 minutes.** Shorter is fine. **(verify the required length)**
- **First 15 seconds: show the product working.** Not your name, not the agenda, not the problem slide. The thing, doing its thing. You can explain after you've earned the attention.
- **Screen recording of the real app.** Not slides. Not a Figma walkthrough. If it isn't real yet, that's what the last week is for.
- **Narrate the user, not the architecture.** "Maria sends an invoice and gets paid in four seconds for a fraction of a cent" — not "the program derives a PDA from the invoice seed."
- **One architecture beat, not five.** Thirty seconds on the interesting technical decision, then back to the product.
- **Land the wedge at the end.** Why this, why now, why you.

Record it more than once. The third take is much better than the first, and it
costs twenty minutes.

## The README

Assume a judge skims it in 60 seconds after the video.

- Problem in two sentences. Concrete, with a named user.
- What you built, and a link to it deployed and clickable.
- **Why Solana** — a paragraph, on the record. Fees, latency, composability, or a primitive that only exists here.
- Architecture: one diagram.
- What's real vs. what's stubbed. **Say this honestly.** Judges find the gaps anyway, and volunteering them buys credibility.
- The team, and why you specifically.

## Deploy it

Devnet at minimum. **Mainnet if you can afford the SOL** — it's a real
differentiator, because it proves the thing runs and that you were willing to
spend to prove it. Put the program ID and a clickable link in the README.

## What separates the top 20

Having watched how these get judged, the pattern is consistent:

1. **It works.** Sounds trivial. A large fraction of submissions don't run.
2. **It's narrow.** One use case, done completely, beats five done partially.
3. **Solana is load-bearing.** The most common quiet killer is a project that would work identically as a web2 app. Judges spot it instantly and it drops you out of contention no matter how well built it is.
4. **A real user exists.** Even one. "I built this because my friend who does X was doing Y by hand" is worth more than a market-size slide.
5. **The founder is credible on the market.** This is an investment decision. Founder-market fit shows up in how you talk about the problem.

## Predictable failure modes

- **Submitting at the deadline.** Uploads fail, videos don't render. Submit a day early; you can usually update.
- **Building through the final week.** Reserve it. A good product with a bad video loses to the reverse.
- **The feature list.** Five half-features read as one unfinished product. Cut ruthlessly and make the happy path immaculate.
- **Hiding the gaps.** Judges test the edges. Stated limitations read as engineering maturity; discovered ones read as overclaiming.
- **Fresh repo, dumped on the last day.** A commit history showing steady work is a signal in itself.

## Before you submit — checklist

- [ ] Video under the limit, product visible in the first 15 seconds
- [ ] README answers "why Solana" explicitly
- [ ] Deployed link works from a clean browser with a fresh wallet
- [ ] Repo public, builds from a clean clone, README says how
- [ ] Program ID and cluster stated
- [ ] Limitations section written honestly
- [ ] Someone outside the team watched the demo and understood it without you talking
- [ ] Submitted at least 24h early
