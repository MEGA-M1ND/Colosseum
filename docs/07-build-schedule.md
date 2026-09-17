# Build schedule: 17 Sep → 12 Oct

25 days. The deadline is **12 October** — confirmed by the user, and it
supersedes the Nov 2 date the earlier plan was built on.

## The situation

The program and frontend already exist and are tested. What does not exist is
**a single line of evidence that any of it runs on a real chain.** Everything
was built in a sandbox with no route to Solana, so the entire deploy path is
unexercised.

That makes week 1 unusual: the work is not building, it is finding out what
breaks. Budget for it to break.

## Week 1 · 17–23 Sep — Make it real

The only week that matters. Nothing else can start until this lands.

- [ ] `./scripts/setup-solana-dev.sh` on your own machine
- [ ] `anchor build` — first time this has ever run. Expect friction: the program is pinned to Anchor 0.31.1 and the toolchain must match
- [ ] `anchor test` — the 23 tests already pass natively; confirm they pass under the real runtime too
- [ ] `anchor deploy --provider.cluster devnet`. Record the program ID and the SOL it cost
- [ ] Replace the placeholder `declare_id!` with the deployed ID, rebuild, redeploy
- [ ] Create a test mint and token accounts; fund the vault
- [ ] Walk the frontend against live devnet end to end — every button
- [ ] Re-run `npm run check:wire` after any program change

**Done when:** you have watched the agent get refused on-chain, in a browser,
with your own eyes. Not before.

## Week 2 · 24–30 Sep — Make it a product

Right now the "agent" is a button in a web page. That is the biggest gap
between this and a winning submission — see **The agent gap** below.

- [ ] Build a real agent loop: an LLM with tools, making its own spending decisions
- [ ] Make the prompt injection genuine — a hostile string in a tool result the agent actually reads and acts on
- [ ] Error states in the UI: every rejection should read as the guard working, never as a crash
- [ ] Deploy the frontend somewhere with a URL (Vercel or similar)

## Week 3 · 1–7 Oct — Make it good

- [ ] Mainnet deploy if the SOL is affordable. A live mainnet program is a real differentiator
- [ ] Write the submission README (draft already at [`SUBMISSION.md`](../SUBMISSION.md))
- [ ] Polish the happy path until it is immaculate; ignore every other path
- [ ] **Record a first cut of the video this week**, not next
- [ ] Show the demo to someone outside the project. Watch where they get confused

## Final · 8–12 Oct — Ship

- [ ] Final video cut against [`08-demo-video.md`](08-demo-video.md)
- [ ] README final, limitations stated honestly
- [ ] Clean clone, build from scratch, confirm the instructions work
- [ ] **Submit on 10 Oct.** Not the 12th

Submitting two days early costs nothing and removes the failure mode that
actually ends hackathon runs: an upload that dies at 3am on deadline night. You
can usually keep updating after submitting.

## The agent gap

The strongest thing in this project is the security argument, and the demo
currently makes it with a button labelled "drain the vault". A judge reads that
as a mock.

The submission gets materially stronger if a **real** agent — an LLM with tools,
choosing its own actions — is the thing that gets injected and refused. The
difference between "we simulated an attack" and "our agent was actually
compromised and the chain stopped it" is most of the pitch.

This is a week-2 build and it is not large: a tool-using loop, a few tools, one
poisoned tool result. Worth prioritising over any additional on-chain feature.

## Prior work — disclose it

The program was written before the hackathon opened. The rules on that could
never be read from the build environment.

**Do not let this sit unresolved.** Read the official rules this week. Either
way, the safe move is the same: state plainly in the README what existed before
the hackathon started and what was built during it. The commit history is
public and dated — a reviewer will see it regardless, so volunteering it reads
as integrity and hiding it reads as the opposite.

## What is already done

Not everything is ahead of you. Already built and tested:

- The on-chain program: 9 instructions, 23 passing tests
- Three attacks found, closed, and kept as tests
- A frontend whose wire format is verified byte-for-byte against the program
- Design docs, a decision log, and the security reasoning written down

The remaining work is mostly proving it runs and telling the story well.
