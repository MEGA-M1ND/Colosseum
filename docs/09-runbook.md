# Runbook: getting it running on your machine

Everything here needs network access to Solana, which the environment this was
built in never had. This is the first time any of it runs for real — expect
friction and budget an afternoon.

macOS or Linux. On Windows use WSL2, not PowerShell.

## 1. Clone and install the toolchain

```bash
git clone https://github.com/MEGA-M1ND/Colosseum.git
cd Colosseum
git checkout claude/colosseum-copilot-setup-6malur

./scripts/setup-solana-dev.sh
```

Installs Rust, the Solana CLI, and **Anchor 0.31.1 specifically**, then points
you at devnet and makes a keypair. Add the Solana binaries to your shell profile
when it tells you to, then reopen the terminal.

Check all four:

```bash
rustc --version && solana --version && anchor --version && node --version
```

`anchor --version` must say **0.31.1**. If it says 1.x the build will fail with
confusing type errors — fix it with `avm install 0.31.1 && avm use 0.31.1`.

## 2. Build and test

```bash
anchor build
```

The first real compile. If it fails, it is almost certainly the Anchor version
or a missing platform-tools download — read the error before changing code.

```bash
cargo test -p agent-wallet      # 23 tests, no validator needed
anchor test                     # full runtime, spins up a local validator
```

`cargo test` passing again tells you the toolchain is sane. `anchor test` is the
new information.

## 3. Take the program's real address

`anchor build` generates a keypair for the program. The source still carries a
placeholder ID, so sync it:

```bash
anchor keys sync
anchor build
```

This rewrites `declare_id!` in `programs/agent-wallet/src/lib.rs` and the ID in
`Anchor.toml`. **Three other places carry that ID** and will now disagree:

```bash
# regenerate the client's reference vectors from the program
cargo test -p agent-wallet --test wire

# update the frontend's copy
#   app/src/lib/program.ts  ->  PROGRAM_ID
```

Then prove they agree:

```bash
cd app && npm install && npm run check:wire
```

That check compares the program ID, so a mismatch fails here rather than on
devnet with an opaque error. **Re-run it after any program change.**

## 4. Deploy to devnet

```bash
solana config set --url devnet
solana airdrop 2          # rate-limited; https://faucet.solana.com is the backup
solana balance

anchor deploy --provider.cluster devnet
```

Deploys cost real SOL and cost it again on every redeploy. Note what it cost —
you will want that number when budgeting a mainnet deploy.

Record the program ID. It goes in `SUBMISSION.md`.

## 5. Make a token to play with

The demo needs a mint, a token account for you, and one owned by the vault PDA.

```bash
# a test token
spl-token create-token --decimals 6
#   -> MINT

spl-token create-account <MINT>
spl-token mint <MINT> 1000

# the vault PDA address is printed in the app once your wallet is connected,
# or derive it: seeds ["vault", ownerPubkey] over the program id
spl-token create-account <MINT> --owner <VAULT_PDA>
```

If the CLI refuses to create an account for the PDA, add
`--allow-non-system-account-owner` — a PDA is not a system account.

Then use the app's **Deposit** button to fund the vault, so you exercise the
program's own path rather than a plain transfer.

## 6. Run the frontend

```bash
cd app
npm install
npm run dev          # http://localhost:5173
```

Point it at devnet explicitly if needed:

```bash
echo 'VITE_RPC_ENDPOINT=https://api.devnet.solana.com' > .env.local
```

You need a browser wallet (Phantom, Solflare) set to **devnet**, holding the
same keypair you have been funding — or fund the browser wallet separately.

## 7. Walk the whole path

In order, and do not skip ahead when something half-works:

1. Connect wallet → **Create vault**
2. Paste the mint → **Deposit**
3. **Generate session key** → **Fund fees** (the agent pays its own)
4. Set a policy → **Grant budget**
5. **Agent pays** a destination → the meter moves
6. **Drain the vault** → must fail, log names the limit
7. **Approve an outside spender** → must fail with `AuthorityAltered`
8. **Revoke** → the agent's next call fails

**Step 7 is the one that matters.** It is the attack a spend limit alone would
not catch, and it is the centre of the demo video.

## When it breaks

- **`anchor build` fails on types** — Anchor version. Must be 0.31.1.
- **`DeclaredProgramIdMismatch`** — you skipped `anchor keys sync`, or deployed before rebuilding after it.
- **Wallet says "unknown program"** — the frontend's `PROGRAM_ID` does not match what you deployed. `npm run check:wire` catches this.
- **`AccountNotInitialized`** — the vault or its token account does not exist yet. Create vault, then the ATA.
- **Airdrop fails** — public faucet rate limit, not your setup. Use faucet.solana.com or wait.
- **Transaction simulation failed, no reason** — run `solana logs` in another terminal and retry. The program's `msg!` output lands there.

## Colosseum Copilot

The skill is already in this repo (`.agents/skills/colosseum-copilot/`), so
cloning gives it to you. It only needs the environment:

```bash
export COLOSSEUM_COPILOT_API_BASE="https://copilot.colosseum.com/api/v1"
export COLOSSEUM_COPILOT_PAT="your-token"

curl "$COLOSSEUM_COPILOT_API_BASE/status" -H "Authorization: Bearer $COLOSSEUM_COPILOT_PAT"
```

Expect `{"authenticated": true, ...}`. Then start Claude Code in this repo and
ask it directly:

> Spend-limited delegation for AI agents on Solana — has anyone built this?
> What does the competitive landscape look like?

> What session key or agent wallet projects have been submitted to past
> Colosseum hackathons?

If it finds a direct competitor, that changes how the wedge should be
positioned — worth knowing before the video is recorded, not after.
