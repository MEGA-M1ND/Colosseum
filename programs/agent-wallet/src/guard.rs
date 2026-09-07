//! ## Guard
//!
//! The phase 2 machinery, ported from `spike/phase2-balance-delta/` where each
//! rule was first demonstrated as a live exploit and then as a rejection.
//!
//! The principle: `agent_invoke` never parses the instruction it executes. It
//! measures the vault's state before and after and judges the difference. That
//! is what lets it work against programs that did not exist when it was
//! written - and it means every field left unmeasured is a hole. The spike
//! found three: authority grants, sibling mints, and native SOL.

use anchor_lang::prelude::*;

use crate::errors::AgentWalletError;

// ## SPL token account layout
//
// Read directly rather than deserializing: after a CPI a typed Anchor account
// is a stale snapshot until reloaded, and a stale read here would report a zero
// delta for every call and wave everything through.
//
//   [0..32]    mint
//   [32..64]   owner
//   [64..72]   amount           (u64 LE)
//   [72..108]  delegate         (COption tag + pubkey)
//   [108]      state
//   [121..129] delegated_amount (u64 LE)
//   [129..165] close_authority  (COption tag + pubkey)

pub const TOKEN_ACCOUNT_LEN: usize = 165;
pub const FINGERPRINT_LEN: usize = 112;

pub fn token_owner(ai: &AccountInfo) -> Result<Pubkey> {
    let raw = ai.try_borrow_data()?;
    require!(
        raw.len() >= TOKEN_ACCOUNT_LEN,
        AgentWalletError::NotVaultTokenAccount
    );
    Ok(Pubkey::new_from_array(raw[32..64].try_into().unwrap()))
}

pub fn token_amount(ai: &AccountInfo) -> Result<u64> {
    let raw = ai.try_borrow_data()?;
    require!(
        raw.len() >= TOKEN_ACCOUNT_LEN,
        AgentWalletError::NotVaultTokenAccount
    );
    Ok(u64::from_le_bytes(raw[64..72].try_into().unwrap()))
}

/// ## Authority fingerprint
///
/// Every field granting standing power over the account, concatenated.
/// `amount` is deliberately excluded - that one is allowed to change, and the
/// balance delta is what bounds it.
///
/// This is what closes the `approve` hole. A balance delta measures value
/// LEAVING; `approve` and `set_authority` grant the right to take it later, at
/// zero delta. Rather than blocklist those instructions - which would forfeit
/// the design's whole claim - snapshot the fields and require them unchanged.
/// Still measuring state, still not interpreting intent.
pub fn authority_fingerprint(ai: &AccountInfo) -> Result<[u8; FINGERPRINT_LEN]> {
    let raw = ai.try_borrow_data()?;
    require!(
        raw.len() >= TOKEN_ACCOUNT_LEN,
        AgentWalletError::NotVaultTokenAccount
    );
    let mut fp = [0u8; FINGERPRINT_LEN];
    fp[0..32].copy_from_slice(&raw[32..64]); // owner
    fp[32..68].copy_from_slice(&raw[72..108]); // delegate
    fp[68..76].copy_from_slice(&raw[121..129]); // delegated_amount
    fp[76..112].copy_from_slice(&raw[129..165]); // close_authority
    Ok(fp)
}

/// One vault-owned account under observation across the CPI.
pub struct Snapshot<'a, 'info> {
    pub account: &'a AccountInfo<'info>,
    pub amount: u64,
    pub fingerprint: [u8; FINGERPRINT_LEN],
    pub lamports: u64,
}

impl<'a, 'info> Snapshot<'a, 'info> {
    pub fn take(account: &'a AccountInfo<'info>) -> Result<Self> {
        Ok(Self {
            amount: token_amount(account)?,
            fingerprint: authority_fingerprint(account)?,
            lamports: account.lamports(),
            account,
        })
    }

    /// Re-measure and enforce the invariants that hold for every vault account,
    /// returning how much left it. `metered` accounts are allowed to fall (the
    /// caller bounds by how much); the rest may grow but must not shrink.
    pub fn enforce(&self, metered: bool) -> Result<u64> {
        require!(
            authority_fingerprint(self.account)? == self.fingerprint,
            AgentWalletError::AuthorityAltered
        );
        require!(
            self.account.lamports() >= self.lamports,
            AgentWalletError::VaultLamportsFell
        );

        let now = token_amount(self.account)?;
        if !metered {
            require!(now >= self.amount, AgentWalletError::UnmeteredHoldingFell);
            return Ok(0);
        }
        Ok(self.amount.saturating_sub(now))
    }
}

/// ## Scope every other vault holding the CPI can reach
///
/// A cap on one account says nothing about the vault's other holdings. A CPI
/// can only touch accounts passed to it, so scanning the forwarded list is
/// complete - anything the instruction could drain is in here.
///
/// The token program is learned from the metered account's own `owner` rather
/// than hardcoded, so the guard still knows nothing it was not told.
pub fn scope_siblings<'a, 'info>(
    forwarded: &'a [AccountInfo<'info>],
    metered: &AccountInfo<'info>,
    vault: &Pubkey,
) -> Result<Vec<Snapshot<'a, 'info>>> {
    let token_program = *metered.owner;
    let mut out: Vec<Snapshot<'a, 'info>> = Vec::new();

    for account in forwarded.iter() {
        if account.key == metered.key {
            continue; // the metered account, bounded by the cap instead
        }
        if out.iter().any(|s| s.account.key == account.key) {
            continue; // duplicate entry in the account list
        }
        if *account.owner != token_program {
            continue; // not a token account
        }
        if account.try_borrow_data()?.len() < TOKEN_ACCOUNT_LEN {
            continue; // a mint, or something else entirely
        }
        if token_owner(account)? != *vault {
            continue; // someone else's token account, not ours to protect
        }
        out.push(Snapshot::take(account)?);
    }

    Ok(out)
}
