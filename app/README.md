# Agent Wallet — frontend

The demo surface. An owner funds a vault and sets a policy; an agent works
inside it; then the agent is attacked and the chain refuses.

## Run

```bash
npm install
npm run dev            # http://localhost:5173
npm run build          # typecheck + production bundle
npm run check:wire     # verify the client matches the program
```

Point it at a cluster with `VITE_RPC_ENDPOINT`; it defaults to devnet.

## The wire check is the important one

There is no generated IDL. `anchor build` needs the SBF toolchain, which was
unreachable in the environment this was written in, so every instruction here is
built by hand: discriminators, Borsh encoding, account order, and the account
layouts decoded from chain.

That is a lot of surface where a mistake shows up only at runtime, as an opaque
failure, during a demo. So the Rust test suite emits reference vectors from the
program's own Anchor types, and `npm run check:wire` rebuilds every call in
TypeScript and byte-compares:

```
cargo test -p agent-wallet --test wire   # regenerate wire-reference.json
npm run check:wire                       # compare
```

It checks discriminators, PDA derivation, all nine instructions (bytes and full
account lists), all twenty error codes, and both account decoders. **Run it
after changing either side.** A reordered field or a renamed instruction fails
here instead of on-chain.

## Layout

| Path | Contents |
|---|---|
| `src/lib/program.ts` | PDAs, Borsh encoding, instruction builders |
| `src/lib/accounts.ts` | Decoders for `Vault` and `Delegation` |
| `src/lib/errors.ts` | Program error codes → what a person can act on |
| `src/lib/session.ts` | The agent's browser-held session key |
| `src/components/` | Vault, policy, agent console, activity log |
| `scripts/check-wire.ts` | The cross-check above |

## Demo path

1. Connect the owner wallet, paste a mint, create the vault, deposit.
2. Generate a session key and fund it for fees. This key is the agent; it is
   generated in the browser and never sent anywhere.
3. Grant a budget — per transaction, lifetime, per window, expiry.
4. Have the agent pay someone. Watch the meter move.
5. **Drain the vault.** The agent, prompt-injected, tries to send everything to
   an attacker. The transaction fails on-chain and the log names the limit.
6. **Approve an outside spender.** The subtler one: it moves no tokens, so a
   spend limit alone would not see it. The program's authority check does.
7. Revoke. The agent's next call fails.

Step 6 is the one worth rehearsing — it is the difference between a spending
limit and a guard that measures everything an instruction can change.

## What is not verified

Nothing here has run against a live cluster: no RPC endpoint was reachable from
the environment it was built in. Typecheck, production build, and the wire check
all pass; the transaction paths themselves are unexercised. Walk the demo path
on devnet before relying on it.
