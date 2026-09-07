// ## On-chain account decoding
//
// Hand-written for the same reason the instruction builders are: no IDL. The
// layouts are verified byte-for-byte by `npm run check:wire` against accounts
// serialized by the program itself.

import { PublicKey } from "@solana/web3.js";

const DISCRIMINATOR = 8;

class Reader {
  private offset = 0;
  constructor(private readonly buf: Buffer) {
    this.offset = DISCRIMINATOR;
  }
  pubkey(): PublicKey {
    const k = new PublicKey(this.buf.subarray(this.offset, this.offset + 32));
    this.offset += 32;
    return k;
  }
  u64(): bigint {
    const v = this.buf.readBigUInt64LE(this.offset);
    this.offset += 8;
    return v;
  }
  i64(): bigint {
    const v = this.buf.readBigInt64LE(this.offset);
    this.offset += 8;
    return v;
  }
  bool(): boolean {
    return this.buf[this.offset++] !== 0;
  }
  u8(): number {
    return this.buf[this.offset++];
  }
  vecPubkey(): PublicKey[] {
    const len = this.buf.readUInt32LE(this.offset);
    this.offset += 4;
    return Array.from({ length: len }, () => this.pubkey());
  }
}

export interface VaultAccount {
  owner: PublicKey;
  paused: boolean;
  bump: number;
}

export function decodeVault(data: Buffer): VaultAccount {
  const r = new Reader(data);
  return { owner: r.pubkey(), paused: r.bool(), bump: r.u8() };
}

export interface DelegationAccount {
  vault: PublicKey;
  agent: PublicKey;
  mint: PublicKey;
  perTxLimit: bigint;
  totalLimit: bigint;
  totalSpent: bigint;
  windowDuration: bigint;
  windowLimit: bigint;
  windowSpent: bigint;
  windowStartedAt: bigint;
  expiresAt: bigint;
  revoked: boolean;
  allowedDestinations: PublicKey[];
  allowedPrograms: PublicKey[];
  bump: number;
}

export function decodeDelegation(data: Buffer): DelegationAccount {
  const r = new Reader(data);
  return {
    vault: r.pubkey(),
    agent: r.pubkey(),
    mint: r.pubkey(),
    perTxLimit: r.u64(),
    totalLimit: r.u64(),
    totalSpent: r.u64(),
    windowDuration: r.i64(),
    windowLimit: r.u64(),
    windowSpent: r.u64(),
    windowStartedAt: r.i64(),
    expiresAt: r.i64(),
    revoked: r.bool(),
    allowedDestinations: r.vecPubkey(),
    allowedPrograms: r.vecPubkey(),
    bump: r.u8(),
  };
}

/** Remaining budget under each limit, for the meter. */
export function remaining(d: DelegationAccount, nowSeconds: number) {
  const lifetime = d.totalLimit - d.totalSpent;
  const windowActive =
    d.windowDuration > 0n &&
    BigInt(nowSeconds) < d.windowStartedAt + d.windowDuration;
  const window = d.windowDuration === 0n
    ? null
    : windowActive
      ? d.windowLimit - d.windowSpent
      : d.windowLimit;
  return { lifetime, window, perTx: d.perTxLimit };
}
