// ## Program errors
//
// Codes are verified against the program by `npm run check:wire` - reordering
// the Rust enum would otherwise silently mislabel every rejection.

export const ERROR_CODES = {
  VaultPaused: 6000,
  DelegationRevoked: 6001,
  DelegationExpired: 6002,
  ZeroAmount: 6003,
  ExceedsPerTxLimit: 6004,
  ExceedsTotalLimit: 6005,
  ExceedsWindowLimit: 6006,
  DestinationNotAllowed: 6007,
  ProgramNotAllowed: 6008,
  SelfInvokeForbidden: 6009,
  TargetNotExecutable: 6010,
  MathOverflow: 6011,
  AuthorityAltered: 6012,
  UnmeteredHoldingFell: 6013,
  VaultLamportsFell: 6014,
  NotVaultTokenAccount: 6015,
  MintMismatch: 6016,
  TooManyEntries: 6017,
  InvalidWindow: 6018,
  InvalidExpiry: 6019,
} as const;

export type ErrorName = keyof typeof ERROR_CODES;

/** What the guard actually stopped, in words a person can act on. */
const EXPLANATIONS: Record<ErrorName, string> = {
  VaultPaused: "The vault is paused. Every agent is stopped.",
  DelegationRevoked: "This delegation was revoked.",
  DelegationExpired: "This session key has expired.",
  ZeroAmount: "Amount must be greater than zero.",
  ExceedsPerTxLimit: "Over the per-transaction cap.",
  ExceedsTotalLimit: "Would exceed the lifetime budget.",
  ExceedsWindowLimit: "Rate limit hit for this window.",
  DestinationNotAllowed: "Destination is not on the allowlist.",
  ProgramNotAllowed: "That program is not on the allowlist.",
  SelfInvokeForbidden: "The agent tried to re-enter this program.",
  TargetNotExecutable: "Target program is not executable.",
  MathOverflow: "Arithmetic overflow.",
  AuthorityAltered:
    "The instruction tried to grant standing authority over the vault — an approval or an authority change. It moves no tokens now and everything later.",
  UnmeteredHoldingFell:
    "The instruction moved a token the delegation does not cover.",
  VaultLamportsFell: "The instruction tried to move the vault's SOL.",
  NotVaultTokenAccount: "That token account does not belong to this vault.",
  MintMismatch: "Wrong mint for this delegation.",
  TooManyEntries: "Allowlist is limited to 8 entries.",
  InvalidWindow: "Set a window duration and a window limit together, or neither.",
  InvalidExpiry: "Expiry must be in the future.",
};

const BY_CODE = new Map<number, ErrorName>(
  Object.entries(ERROR_CODES).map(([name, code]) => [code, name as ErrorName]),
);

export interface DecodedError {
  code: number | null;
  name: ErrorName | null;
  message: string;
  /** True when the guard rejected it, rather than the transaction failing. */
  isPolicyRejection: boolean;
}

/**
 * Pull the program's error out of whatever web3.js threw. Simulation failures,
 * send failures, and RPC errors all carry the code in different places.
 */
export function decodeError(err: unknown): DecodedError {
  const code = extractCode(err);
  if (code === null) {
    return {
      code: null,
      name: null,
      message: err instanceof Error ? err.message : String(err),
      isPolicyRejection: false,
    };
  }
  const name = BY_CODE.get(code) ?? null;
  return {
    code,
    name,
    message: name ? EXPLANATIONS[name] : `Program error ${code}.`,
    isPolicyRejection: name !== null,
  };
}

function extractCode(err: unknown): number | null {
  if (!err || typeof err !== "object") return null;

  // SendTransactionError / SimulateTransaction shapes
  const anyErr = err as Record<string, any>;
  const instructionError =
    anyErr?.InstructionError ??
    anyErr?.err?.InstructionError ??
    anyErr?.transactionError?.InstructionError;
  if (Array.isArray(instructionError)) {
    const detail = instructionError[1];
    if (detail && typeof detail === "object" && "Custom" in detail) {
      return Number(detail.Custom);
    }
  }

  // Fall back to the logs, which is where a simulation failure usually lands.
  const logs: string[] | undefined = anyErr.logs;
  const haystack = [anyErr.message, ...(logs ?? [])]
    .filter(Boolean)
    .join("\n");
  const match = haystack.match(/custom program error: 0x([0-9a-fA-F]+)/);
  if (match) return parseInt(match[1], 16);

  return null;
}
