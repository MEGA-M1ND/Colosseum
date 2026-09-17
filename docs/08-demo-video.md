# Demo video

Most judges watch the video and never run the code. It is the submission.

**Target: under 3 minutes** — verify the real limit in the official rules.

## The one rule

**Show the product working in the first 15 seconds.** Not your name, not the
problem slide, not the agenda. Earn the attention first, explain after.

## Shot list

### 0:00–0:15 · The refusal, cold

Open on the moment. An agent tries to move money, and the transaction fails
on-chain with the reason visible.

No setup, no narration beyond one line: *"This agent was just told to drain its
owner's wallet. The blockchain said no."*

Then cut back and explain how you got there.

### 0:15–0:40 · The problem

Agents that spend need a key. A key is all-or-nothing. One prompt injection and
it's gone.

Keep it concrete — a real scenario, a specific loss. Skip the market-size slide.

### 0:40–1:20 · Setting a budget

Screen recording of the real app:

- Owner funds the vault
- Grants the agent $100 total, $10 per transaction, expires in 24 hours
- The session key is generated in the browser — say out loud that it never leaves, and that it is not the owner's key

### 1:20–1:50 · The agent works

Let it do its actual job autonomously. A few small spends. The budget meter
moves.

This beat matters: the product has to be useful before the safety is
interesting. If the agent is a button, this is where a judge notices.

### 1:50–2:20 · The attack

**The centrepiece.** Show the injection arriving — a hostile string in a tool
result the agent reads. The agent believes it and tries to send everything to an
attacker.

The transaction fails. Show the explorer. Name the error.

Then the second attack, and say why it is different: *"This one moves no tokens
at all. It grants the attacker permission to take them later. A spending limit
would never see it."* Show it refused too.

That contrast is the strongest thirty seconds you have. Do not cut it for time.

### 2:20–2:45 · Why it holds

One technical beat, not five: the program never parses the agent's instruction.
It measures what left the vault. That is why it works against programs that did
not exist when it was written.

Mention that you found three ways around your own design and closed all three.

### 2:45–3:00 · Close

What it is in one sentence, who it is for, and why you.

## Production notes

- **Record it more than once.** The third take is much better than the first and costs twenty minutes.
- Screen recording of the real app. Not slides, not Figma.
- Narrate the user, not the architecture. "The agent tries to send everything and fails" — not "the guard re-reads the token account post-CPI."
- Clean browser: no bookmarks bar, no notifications, no unrelated tabs.
- Pre-fund every account and pre-stage every screen. Dead air waiting for a confirmation kills the pacing.
- Rehearse the injection so it is reproducible on camera. Script the hostile string.
- Check the audio. Bad audio reads as low effort faster than bad video does.

## Cut these if you run long

In order: the architecture beat, the second half of the problem section, the
team close. **Never** the attack sequence.
