// ## Agent Wallet client
//
// Instructions are built by hand rather than from an Anchor IDL: `anchor build`
// needs the SBF toolchain, which was unreachable where this was written, so
// there is no generated IDL to import.
//
// That makes the wire format a real risk - a wrong discriminator or a misplaced
// account fails at runtime with an opaque error. `npm run check:wire` byte-
// compares everything here against reference vectors emitted by the Rust
// program's own test suite. Run it after any change to either side.

import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { DISCRIMINATORS } from "./discriminators";

export const PROGRAM_ID = new PublicKey(
  "Ag3ntWa11et11111111111111111111111111111111",
);

export const VAULT_SEED = Buffer.from("vault");
export const DELEGATION_SEED = Buffer.from("delegation");

// ## PDAs

export function vaultPda(owner: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [VAULT_SEED, owner.toBuffer()],
    PROGRAM_ID,
  )[0];
}

export function delegationPda(vault: PublicKey, agent: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [DELEGATION_SEED, vault.toBuffer(), agent.toBuffer()],
    PROGRAM_ID,
  )[0];
}

// ## Borsh encoding
//
// Only the shapes this program uses. Anchor prefixes every instruction with an
// 8-byte discriminator, then Borsh-encodes the arguments in declaration order.

function u64(value: bigint | number): Buffer {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(value));
  return b;
}

function i64(value: bigint | number): Buffer {
  const b = Buffer.alloc(8);
  b.writeBigInt64LE(BigInt(value));
  return b;
}

function vecPubkey(keys: PublicKey[]): Buffer {
  const len = Buffer.alloc(4);
  len.writeUInt32LE(keys.length);
  return Buffer.concat([len, ...keys.map((k) => k.toBuffer())]);
}

function vecU8(bytes: Uint8Array): Buffer {
  const len = Buffer.alloc(4);
  len.writeUInt32LE(bytes.length);
  return Buffer.concat([len, Buffer.from(bytes)]);
}

function bool(v: boolean): Buffer {
  return Buffer.from([v ? 1 : 0]);
}

// ## Policy

export interface Policy {
  perTxLimit: bigint;
  totalLimit: bigint;
  /** Seconds. Zero disables the rate limit. */
  windowDuration: bigint;
  windowLimit: bigint;
  /** Unix seconds. */
  expiresAt: bigint;
  /** Token account OWNERS the agent may send to. Empty means any. */
  allowedDestinations: PublicKey[];
  /** Programs `agentInvoke` may call. Empty means none - invoke is opt-in. */
  allowedPrograms: PublicKey[];
}

export function encodePolicy(p: Policy): Buffer {
  return Buffer.concat([
    u64(p.perTxLimit),
    u64(p.totalLimit),
    i64(p.windowDuration),
    u64(p.windowLimit),
    i64(p.expiresAt),
    vecPubkey(p.allowedDestinations),
    vecPubkey(p.allowedPrograms),
  ]);
}

/** No rate limit, no allowlists, far-future expiry. A starting point to edit. */
export function defaultPolicy(): Policy {
  return {
    perTxLimit: 100n,
    totalLimit: 500n,
    windowDuration: 0n,
    windowLimit: 0n,
    expiresAt: BigInt(Math.floor(Date.now() / 1000) + 86_400),
    allowedDestinations: [],
    allowedPrograms: [],
  };
}

// ## Instructions
//
// Account order must match each `#[derive(Accounts)]` struct exactly. The wire
// check enforces that.

const ro = (pubkey: PublicKey, isSigner = false) => ({
  pubkey,
  isSigner,
  isWritable: false,
});
const rw = (pubkey: PublicKey, isSigner = false) => ({
  pubkey,
  isSigner,
  isWritable: true,
});

export function initializeVault(owner: PublicKey): TransactionInstruction {
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      rw(owner, true),
      rw(vaultPda(owner)),
      ro(SystemProgram.programId),
    ],
    data: DISCRIMINATORS.initializeVault,
  });
}

export function setPaused(
  owner: PublicKey,
  paused: boolean,
): TransactionInstruction {
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [ro(owner, true), rw(vaultPda(owner))],
    data: Buffer.concat([DISCRIMINATORS.setPaused, bool(paused)]),
  });
}

export function createDelegation(
  owner: PublicKey,
  agent: PublicKey,
  mint: PublicKey,
  policy: Policy,
): TransactionInstruction {
  const vault = vaultPda(owner);
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      rw(owner, true),
      ro(vault),
      ro(agent),
      ro(mint),
      rw(delegationPda(vault, agent)),
      ro(SystemProgram.programId),
    ],
    data: Buffer.concat([DISCRIMINATORS.createDelegation, encodePolicy(policy)]),
  });
}

export function updateDelegation(
  owner: PublicKey,
  agent: PublicKey,
  policy: Policy,
): TransactionInstruction {
  const vault = vaultPda(owner);
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [ro(owner, true), ro(vault), rw(delegationPda(vault, agent))],
    data: Buffer.concat([DISCRIMINATORS.updateDelegation, encodePolicy(policy)]),
  });
}

export function revokeDelegation(
  owner: PublicKey,
  agent: PublicKey,
): TransactionInstruction {
  const vault = vaultPda(owner);
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [ro(owner, true), ro(vault), rw(delegationPda(vault, agent))],
    data: DISCRIMINATORS.revokeDelegation,
  });
}

export function deposit(
  owner: PublicKey,
  mint: PublicKey,
  ownerTokenAccount: PublicKey,
  vaultTokenAccount: PublicKey,
  amount: bigint,
): TransactionInstruction {
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      rw(owner, true),
      ro(vaultPda(owner)),
      ro(mint),
      rw(ownerTokenAccount),
      rw(vaultTokenAccount),
      ro(TOKEN_PROGRAM_ID),
    ],
    data: Buffer.concat([DISCRIMINATORS.deposit, u64(amount)]),
  });
}

export function withdraw(
  owner: PublicKey,
  mint: PublicKey,
  vaultTokenAccount: PublicKey,
  destinationTokenAccount: PublicKey,
  amount: bigint,
): TransactionInstruction {
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      ro(owner, true),
      ro(vaultPda(owner)),
      ro(mint),
      rw(vaultTokenAccount),
      rw(destinationTokenAccount),
      ro(TOKEN_PROGRAM_ID),
    ],
    data: Buffer.concat([DISCRIMINATORS.withdraw, u64(amount)]),
  });
}

export interface AgentContext {
  agent: PublicKey;
  vault: PublicKey;
  mint: PublicKey;
  vaultTokenAccount: PublicKey;
}

export function agentTransfer(
  ctx: AgentContext,
  destinationTokenAccount: PublicKey,
  amount: bigint,
): TransactionInstruction {
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      ro(ctx.agent, true),
      ro(ctx.vault),
      rw(delegationPda(ctx.vault, ctx.agent)),
      ro(ctx.mint),
      rw(ctx.vaultTokenAccount),
      rw(destinationTokenAccount),
      ro(TOKEN_PROGRAM_ID),
    ],
    data: Buffer.concat([DISCRIMINATORS.agentTransfer, u64(amount)]),
  });
}

/**
 * Arbitrary CPI, bounded by measurement. `remaining` is the account list the
 * inner instruction needs; the program signs as the vault for any entry whose
 * key is the vault PDA.
 */
export function agentInvoke(
  ctx: AgentContext,
  targetProgram: PublicKey,
  data: Uint8Array,
  remaining: { pubkey: PublicKey; isSigner: boolean; isWritable: boolean }[],
): TransactionInstruction {
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      ro(ctx.agent, true),
      ro(ctx.vault),
      rw(delegationPda(ctx.vault, ctx.agent)),
      ro(ctx.mint),
      rw(ctx.vaultTokenAccount),
      ro(targetProgram),
      ...remaining,
    ],
    data: Buffer.concat([DISCRIMINATORS.agentInvoke, vecU8(data)]),
  });
}

// ## SPL token instruction data, for agentInvoke payloads

export function splTransferData(amount: bigint): Uint8Array {
  return Uint8Array.from([3, ...u64(amount)]);
}

export function splApproveData(amount: bigint): Uint8Array {
  return Uint8Array.from([4, ...u64(amount)]);
}
