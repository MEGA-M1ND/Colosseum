# Solana ramp-up

Ordered by dependency, and filtered to what actually causes problems. If you're
coming from EVM, the first section is the whole game — most Solana confusion is
one wrong mental model applied repeatedly.

## 1. The accounts model

**Programs are stateless.** A program is pure code. It stores nothing. All state
lives in separate accounts that the program owns.

**Every account a transaction touches must be declared up front.** You cannot
discover an account mid-execution and go read it. The caller passes in the full
list, and the runtime uses that list to parallelize — which is where Solana's
throughput comes from.

This is the shift. In Solidity, `balances[msg.sender]` is a lookup the contract
does. On Solana, the client must know which account holds that balance, derive
its address, and pass it in. Your client code carries knowledge your program
would carry on EVM.

**Accounts pay rent.** An account must hold enough SOL to be rent-exempt or it
gets purged. Rent scales with size, so account size is a cost decision. Closing
an account refunds it.

## 2. PDAs (Program Derived Addresses)

An address derived deterministically from seeds + program ID, off the ed25519
curve — so no private key exists for it. Two consequences, both essential:

- **Deterministic state.** `["vault", user_pubkey]` always derives the same address. That's how the client knows what to pass in.
- **Programs can sign.** A program can sign for its own PDAs. That's how a program custodies tokens or authorizes CPI.

Seed design is schema design. Get it right on paper in week 3 — changing seeds
later invalidates every account you've created.

## 3. CPI (Cross-Program Invocation)

Calling another program from yours — transferring SPL tokens, minting, calling
a DEX. With `invoke_signed`, your program signs as its PDA.

Watch for: **arbitrary CPI**. If you don't verify the program ID you're calling
into, an attacker passes a malicious program and your PDA signs for it.

## 4. Anchor

The framework you should use. It gives you:

- `#[derive(Accounts)]` — declarative account validation, with constraints like `has_one`, `seeds`, `bump`, `constraint = ...`. Most security checks become declarations.
- 8-byte discriminators for account and instruction types.
- An IDL, and a generated TypeScript client from it.

Anchor's constraint system is the main reason to use it. Every check you write
by hand is a check you can forget.

## 5. The limits that bite

| Limit | Value | What it means |
|---|---|---|
| Transaction size | **1232 bytes** | The one that surprises people. Accounts are 32 bytes each; a complex instruction runs out of room. Address Lookup Tables extend this. |
| Compute units | 200k default, 1.4M max | Request more explicitly with a compute budget instruction. Loops over accounts get expensive fast. |
| Stack frame | 4KB | Large structs on the stack blow it. Box them. |
| CPI depth | 4 | Limits how deep composition can nest. |

Hit the transaction size limit once early and deliberately, so you recognize it
under pressure.

## 6. Tokens

- **SPL Token** — the original program.
- **Token-2022** — successor, adds extensions: confidential transfers, transfer hooks, metadata, interest-bearing. Not a drop-in; wallets and programs need explicit support. Check support before betting an idea on it.
- **ATAs (Associated Token Accounts)** — the canonical PDA holding a given mint for a given owner. Deriving these and knowing who creates and pays for them is routine work.

## 7. Security footguns

The list that accounts for most real Solana exploits:

- **Missing signer check** — you assumed an account authorized something. Anchor: `Signer<'info>`.
- **Missing owner check** — an attacker passes an account of the right shape owned by a different program. Anchor's typed accounts handle this; raw ones don't.
- **Account substitution** — the right *type* of account but the wrong *instance*. Guard with `has_one` and seed constraints.
- **Arbitrary CPI** — see above. Verify program IDs.
- **Bump seed canonicalization** — accept only the canonical bump, or an attacker creates a second valid PDA for the same seeds.
- **Integer overflow** — use checked arithmetic. Release builds don't panic on overflow by default.
- **Rounding direction** — always round in the protocol's favor. Rounding the user's way is a slow drain.

For an agent-wallet or payments idea, the first four are the ones judges and
any security-minded reviewer will look for.

## 8. Local workflow

```bash
solana-test-validator          # local chain
anchor build                   # compile
anchor test                    # spins up a validator, deploys, runs TS tests
anchor deploy --provider.cluster devnet
solana logs                    # the actual debugging tool
```

`solana logs` is where you'll live. `msg!()` in the program prints there.

## Resources

- **Anchor Book** — the framework reference.
- **Solana Cookbook** — task-shaped recipes; good for "how do I actually do X."
- **Solana Program Library** — read real production programs. The token program is worth reading start to finish.
- **Updraft Solana course** — a full course; Colosseum flagged it in their own 2026 announcement, so it's aligned with what they expect builders to know.
- **Neodyme's Solana security blog + their CTF workshop** — the best material on the footguns above.

## Do this, not just read this

Reading Solana material has poor retention because the mental model only lands
when the runtime rejects your transaction. Week 1 of the prep plan is
deliberately a build task, not a reading list. Follow it in that order.
