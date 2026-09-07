//! ## Errors
//!
//! Distinct codes per failure mode, so a rejection says what stopped it and a
//! test cannot pass for the wrong reason. The guard codes mirror the spike:
//! authority altered, unmetered holding fell, vault lamports fell.

use anchor_lang::prelude::*;

#[error_code]
pub enum AgentWalletError {
    #[msg("Vault is paused by its owner")]
    VaultPaused,
    #[msg("Delegation has been revoked")]
    DelegationRevoked,
    #[msg("Delegation has expired")]
    DelegationExpired,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Amount exceeds the per-transaction limit")]
    ExceedsPerTxLimit,
    #[msg("Spend would exceed the delegation's lifetime limit")]
    ExceedsTotalLimit,
    #[msg("Spend would exceed the rate limit for this window")]
    ExceedsWindowLimit,
    #[msg("Destination is not on the delegation's allowlist")]
    DestinationNotAllowed,
    #[msg("Target program is not on the delegation's allowlist")]
    ProgramNotAllowed,
    #[msg("Target program must not be this program")]
    SelfInvokeForbidden,
    #[msg("Target program is not executable")]
    TargetNotExecutable,
    #[msg("Arithmetic overflow")]
    MathOverflow,
    #[msg("Instruction altered an authority field on a vault token account")]
    AuthorityAltered,
    #[msg("An unmetered vault holding decreased")]
    UnmeteredHoldingFell,
    #[msg("Lamports left the vault")]
    VaultLamportsFell,
    #[msg("Token account does not belong to this vault")]
    NotVaultTokenAccount,
    #[msg("Token account is for the wrong mint")]
    MintMismatch,
    #[msg("Too many entries for the allowlist")]
    TooManyEntries,
    #[msg("Window duration and limit must be set together")]
    InvalidWindow,
    #[msg("Expiry must be in the future")]
    InvalidExpiry,
}
