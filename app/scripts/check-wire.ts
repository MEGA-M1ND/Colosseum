// ## Wire format check
//
// The client builds instructions by hand, so nothing catches a wrong
// discriminator or a misordered account until it fails on-chain with an opaque
// error. This rebuilds every instruction in TypeScript from the same fixed
// inputs the Rust test used and byte-compares against its output.
//
// Regenerate the reference with:
//   cargo test -p agent-wallet --test wire

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { PublicKey } from "@solana/web3.js";
import * as aw from "../src/lib/program";
import { DISCRIMINATORS } from "../src/lib/discriminators";
import { ERROR_CODES } from "../src/lib/errors";
import { decodeDelegation, decodeVault } from "../src/lib/accounts";

const ref = JSON.parse(
  readFileSync(new URL("../wire-reference.json", import.meta.url), "utf8"),
);

let failures = 0;
const fail = (msg: string) => {
  console.error(`  FAIL  ${msg}`);
  failures++;
};
const pass = (msg: string) => console.log(`  ok    ${msg}`);

// ## 1. Discriminators recompute correctly

console.log("\ndiscriminators");
const NAMES: Record<string, string> = {
  initializeVault: "initialize_vault",
  setPaused: "set_paused",
  createDelegation: "create_delegation",
  updateDelegation: "update_delegation",
  revokeDelegation: "revoke_delegation",
  deposit: "deposit",
  withdraw: "withdraw",
  agentTransfer: "agent_transfer",
  agentInvoke: "agent_invoke",
};
for (const [camel, snake] of Object.entries(NAMES)) {
  const want = createHash("sha256")
    .update(`global:${snake}`)
    .digest()
    .subarray(0, 8);
  const got = (DISCRIMINATORS as Record<string, Buffer>)[camel];
  if (!got || !want.equals(got)) {
    fail(`${camel}: constant ${got?.toString("hex")} != sha256 ${want.toString("hex")}`);
  } else {
    pass(camel);
  }
}

// ## 2. PDAs match the program's derivation

console.log("\npdas");
const owner = new PublicKey(ref.inputs.owner);
const agent = new PublicKey(ref.inputs.agent);
const mint = new PublicKey(ref.inputs.mint);
const ownerToken = new PublicKey(ref.inputs.ownerToken);
const vaultToken = new PublicKey(ref.inputs.vaultToken);
const destToken = new PublicKey(ref.inputs.destToken);
const target = new PublicKey(ref.inputs.target);

const vault = aw.vaultPda(owner);
const delegation = aw.delegationPda(vault, agent);
vault.toBase58() === ref.pdas.vault
  ? pass("vault")
  : fail(`vault: ${vault.toBase58()} != ${ref.pdas.vault}`);
delegation.toBase58() === ref.pdas.delegation
  ? pass("delegation")
  : fail(`delegation: ${delegation.toBase58()} != ${ref.pdas.delegation}`);

if (aw.PROGRAM_ID.toBase58() !== ref.programId) {
  fail(`programId: ${aw.PROGRAM_ID.toBase58()} != ${ref.programId}`);
} else {
  pass("programId");
}

// ## 3. Every instruction, byte for byte

const policy: aw.Policy = {
  perTxLimit: BigInt(ref.policy.perTxLimit),
  totalLimit: BigInt(ref.policy.totalLimit),
  windowDuration: BigInt(ref.policy.windowDuration),
  windowLimit: BigInt(ref.policy.windowLimit),
  expiresAt: BigInt(ref.policy.expiresAt),
  allowedDestinations: ref.policy.allowedDestinations.map(
    (k: string) => new PublicKey(k),
  ),
  allowedPrograms: ref.policy.allowedPrograms.map(
    (k: string) => new PublicKey(k),
  ),
};

const agentCtx: aw.AgentContext = {
  agent,
  vault,
  mint,
  vaultTokenAccount: vaultToken,
};

const built: Record<string, ReturnType<typeof aw.initializeVault>> = {
  initializeVault: aw.initializeVault(owner),
  setPaused: aw.setPaused(owner, true),
  createDelegation: aw.createDelegation(owner, agent, mint, policy),
  updateDelegation: aw.updateDelegation(owner, agent, policy),
  revokeDelegation: aw.revokeDelegation(owner, agent),
  deposit: aw.deposit(owner, mint, ownerToken, vaultToken, 250n),
  withdraw: aw.withdraw(owner, mint, vaultToken, destToken, 900n),
  agentTransfer: aw.agentTransfer(agentCtx, destToken, 50n),
  agentInvoke: aw.agentInvoke(agentCtx, target, aw.splTransferData(50n), [
    { pubkey: vaultToken, isSigner: false, isWritable: true },
    { pubkey: destToken, isSigner: false, isWritable: true },
    { pubkey: vault, isSigner: false, isWritable: false },
  ]),
};

console.log("\ninstructions");
for (const expected of ref.instructions) {
  const name: string = expected.name;
  const ix = built[name];
  if (!ix) {
    fail(`${name}: not built by the client`);
    continue;
  }

  const gotData = Buffer.from(ix.data).toString("hex");
  if (gotData !== expected.data) {
    fail(`${name} data:\n        got  ${gotData}\n        want ${expected.data}`);
    continue;
  }

  const gotKeys = ix.keys.map((k) => ({
    pubkey: k.pubkey.toBase58(),
    isSigner: k.isSigner,
    isWritable: k.isWritable,
  }));
  const wantKeys = expected.accounts;

  if (gotKeys.length !== wantKeys.length) {
    fail(`${name} accounts: got ${gotKeys.length}, want ${wantKeys.length}`);
    continue;
  }

  let mismatch = false;
  for (let i = 0; i < gotKeys.length; i++) {
    const g = gotKeys[i];
    const w = wantKeys[i];
    if (
      g.pubkey !== w.pubkey ||
      g.isSigner !== w.isSigner ||
      g.isWritable !== w.isWritable
    ) {
      fail(
        `${name} account[${i}]:\n        got  ${JSON.stringify(g)}\n        want ${JSON.stringify(w)}`,
      );
      mismatch = true;
      break;
    }
  }
  if (!mismatch) pass(`${name} (${ix.keys.length} accounts, ${ix.data.length}B)`);
}

// ## 4. Error codes match the program's enum

console.log("\nerror codes");
for (const [name, code] of Object.entries(ref.errors as Record<string, number>)) {
  const got = (ERROR_CODES as Record<string, number>)[name];
  if (got === undefined) {
    fail(`${name}: missing from the client's error map`);
  } else if (got !== code) {
    fail(`${name}: client says ${got}, program says ${code}`);
  } else {
    pass(`${name} = ${code}`);
  }
}
for (const name of Object.keys(ERROR_CODES)) {
  if (!(name in ref.errors)) fail(`${name}: in the client but not the program`);
}

// ## 5. Account layouts decode what the program serialized

console.log("\naccount decoding");
{
  const d = decodeDelegation(Buffer.from(ref.accounts.delegation.hex, "hex"));
  const want = ref.accounts.delegation.fields;
  const checks: [string, unknown, unknown][] = [
    ["vault", d.vault.toBase58(), want.vault],
    ["agent", d.agent.toBase58(), want.agent],
    ["mint", d.mint.toBase58(), want.mint],
    ["perTxLimit", Number(d.perTxLimit), want.perTxLimit],
    ["totalLimit", Number(d.totalLimit), want.totalLimit],
    ["totalSpent", Number(d.totalSpent), want.totalSpent],
    ["windowDuration", Number(d.windowDuration), want.windowDuration],
    ["windowLimit", Number(d.windowLimit), want.windowLimit],
    ["windowSpent", Number(d.windowSpent), want.windowSpent],
    ["windowStartedAt", Number(d.windowStartedAt), want.windowStartedAt],
    ["expiresAt", Number(d.expiresAt), want.expiresAt],
    ["revoked", d.revoked, want.revoked],
    ["allowedDestinations", d.allowedDestinations.map((k) => k.toBase58()).join(","), want.allowedDestinations.join(",")],
    ["allowedPrograms", d.allowedPrograms.map((k) => k.toBase58()).join(","), want.allowedPrograms.join(",")],
    ["bump", d.bump, want.bump],
  ];
  for (const [field, got, expect] of checks) {
    got === expect
      ? pass(`delegation.${field}`)
      : fail(`delegation.${field}: got ${got}, want ${expect}`);
  }

  const v = decodeVault(Buffer.from(ref.accounts.vault.hex, "hex"));
  const wv = ref.accounts.vault.fields;
  v.owner.toBase58() === wv.owner ? pass("vault.owner") : fail("vault.owner");
  v.paused === wv.paused ? pass("vault.paused") : fail("vault.paused");
  v.bump === wv.bump ? pass("vault.bump") : fail("vault.bump");
}

console.log(
  failures === 0
    ? "\nwire format matches the program\n"
    : `\n${failures} mismatch(es)\n`,
);
process.exit(failures === 0 ? 0 : 1);
