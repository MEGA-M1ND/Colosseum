// ## Anchor instruction discriminators
//
// sha256("global:<snake_case_name>").slice(0, 8), the Anchor 0.31 scheme.
//
// Hardcoded rather than computed at runtime so the browser bundle needs no
// hashing. Generated and verified by `npm run check:wire`, which recomputes
// them AND byte-compares full instructions against reference vectors emitted
// by the Rust test suite. Do not hand-edit.

export const DISCRIMINATORS = {
  initializeVault: Buffer.from([48, 191, 163, 44, 71, 129, 63, 164]),
  setPaused: Buffer.from([91, 60, 125, 192, 176, 225, 166, 218]),
  createDelegation: Buffer.from([177, 165, 93, 55, 227, 163, 61, 175]),
  updateDelegation: Buffer.from([87, 91, 130, 42, 18, 37, 155, 70]),
  revokeDelegation: Buffer.from([188, 92, 135, 67, 160, 181, 54, 62]),
  deposit: Buffer.from([242, 35, 198, 137, 82, 225, 242, 182]),
  withdraw: Buffer.from([183, 18, 70, 156, 148, 109, 161, 34]),
  agentTransfer: Buffer.from([199, 111, 151, 49, 124, 13, 150, 44]),
  agentInvoke: Buffer.from([131, 116, 147, 221, 201, 18, 194, 112]),
} as const;
