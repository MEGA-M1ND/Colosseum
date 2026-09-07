//! ## Phase 2 spike — balance-delta enforcement over arbitrary CPI
//!
//! Question: can a program let an agent invoke an ARBITRARY instruction as the
//! vault, without parsing that instruction, and still bound how much value
//! leaves the vault?
//!
//! Method: read the vault token balance, `invoke_signed` whatever the caller
//! asked for, read the balance again, and reject if the drop exceeds the cap.
//! Returning Err reverts the CPI atomically, so measuring after the fact is safe.

use solana_program::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
};

// ## Constants

pub const VAULT_SEED: &[u8] = b"vault";

// ## SPL token account layout
//
// Read the fields directly rather than going through Pack: spl-token and
// solana-program pull in different solana-program-pack versions, and the
// layout is fixed and stable anyway.
//   [0..32]    mint
//   [32..64]   owner
//   [64..72]   amount (u64 LE)
//   [72..108]  delegate       (COption tag u32 LE + pubkey)
//   [108]      state
//   [121..129] delegated_amount (u64 LE)
//   [129..165] close_authority (COption tag u32 LE + pubkey)

fn token_owner(ai: &AccountInfo) -> Result<Pubkey, ProgramError> {
    let raw = ai.try_borrow_data()?;
    if raw.len() < 72 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(Pubkey::new_from_array(raw[32..64].try_into().unwrap()))
}

fn token_amount(ai: &AccountInfo) -> Result<u64, ProgramError> {
    let raw = ai.try_borrow_data()?;
    if raw.len() < 72 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(u64::from_le_bytes(raw[64..72].try_into().unwrap()))
}

// ## Authority fingerprint
//
// Every field on the token account that grants someone standing power over it,
// concatenated. `amount` is deliberately excluded - that one is allowed to
// change, and the delta check is what bounds it.
//
// This is the fix for the `approve` hole. A balance delta measures value
// LEAVING; `approve` and `set_authority` grant the right to take it later, at
// zero delta. Rather than start parsing instructions - which would forfeit the
// whole point of the design - snapshot these fields and require they come back
// untouched. Still measuring state, still not interpreting intent.

const FINGERPRINT_LEN: usize = 112;

fn authority_fingerprint(ai: &AccountInfo) -> Result<[u8; FINGERPRINT_LEN], ProgramError> {
    let raw = ai.try_borrow_data()?;
    if raw.len() < 165 {
        return Err(ProgramError::InvalidAccountData);
    }
    let mut fp = [0u8; FINGERPRINT_LEN];
    fp[0..32].copy_from_slice(&raw[32..64]);     // owner
    fp[32..68].copy_from_slice(&raw[72..108]);   // delegate
    fp[68..76].copy_from_slice(&raw[121..129]);  // delegated_amount
    fp[76..112].copy_from_slice(&raw[129..165]); // close_authority
    Ok(fp)
}

// ## Instruction data
//
// [0..8]  spend cap, u64 LE
// [8..]   opaque instruction data forwarded to the target program

// ## Accounts
//
// 0        vault authority PDA (signs via seeds; never a real keypair)
// 1        vault token account (the one account we meter)
// 2        target program to invoke
// 3..      accounts forwarded to the target instruction

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> Result<(), ProgramError> {
    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let cap = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let inner_data = data[8..].to_vec();

    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let vault_authority = &accounts[0];
    let vault_token_account = &accounts[1];
    let target_program = &accounts[2];
    let forwarded = &accounts[3..];

    // ## Verify the vault PDA is ours
    let (expected_vault, bump) = Pubkey::find_program_address(&[VAULT_SEED], program_id);
    if expected_vault != *vault_authority.key {
        return Err(ProgramError::InvalidSeeds);
    }

    // ## Verify the metered account is the vault's, and read the BEFORE balance
    if token_owner(vault_token_account)? != *vault_authority.key {
        msg!("metered account is not owned by the vault authority");
        return Err(ProgramError::IllegalOwner);
    }
    let before = token_amount(vault_token_account)?;
    let authority_before = authority_fingerprint(vault_token_account)?;
    msg!("balance before: {}", before);

    // ## Snapshot every OTHER vault-owned token account the CPI can reach
    //
    // A cap on one account says nothing about the vault's other holdings. A CPI
    // can only touch accounts that were passed to it, so scanning the forwarded
    // set is complete: anything the instruction could drain is in here.
    //
    // The token program is learned from the metered account's owner rather than
    // hardcoded, so the guard still knows nothing it wasn't told.
    let token_program_id = *vault_token_account.owner;
    let mut siblings: Vec<(&AccountInfo, u64, [u8; FINGERPRINT_LEN])> = Vec::new();
    for a in forwarded.iter() {
        if a.key == vault_token_account.key {
            continue; // the metered account, handled by the cap below
        }
        if siblings.iter().any(|(s, _, _)| s.key == a.key) {
            continue; // duplicate entry in the account list
        }
        if *a.owner != token_program_id {
            continue; // not a token account
        }
        if a.try_borrow_data()?.len() < 165 {
            continue; // a mint, or something else entirely
        }
        if token_owner(a)? != *vault_authority.key {
            continue; // someone else's token account - not ours to protect
        }
        siblings.push((a, token_amount(a)?, authority_fingerprint(a)?));
    }
    msg!("sibling vault accounts in scope: {}", siblings.len());

    // ## Build and invoke the caller's instruction, unparsed
    //
    // The guard has no idea what this instruction does. That is the point: it
    // works against programs that did not exist when this was written.
    let metas: Vec<AccountMeta> = forwarded
        .iter()
        .map(|a| AccountMeta {
            pubkey: *a.key,
            // The vault PDA is the only thing we sign for.
            is_signer: a.key == vault_authority.key,
            is_writable: a.is_writable,
        })
        .collect();

    let ix = Instruction {
        program_id: *target_program.key,
        accounts: metas,
        data: inner_data,
    };

    let mut infos = forwarded.to_vec();
    infos.push(target_program.clone());

    invoke_signed(&ix, &infos, &[&[VAULT_SEED, &[bump]]])?;

    // ## Read the AFTER balance
    //
    // The runtime writes CPI results back into the shared account data, so a
    // fresh borrow sees post-CPI state. If this read were stale the entire
    // approach would be unsound - that is the thing this spike proves.
    let after = token_amount(vault_token_account)?;
    msg!("balance after: {}", after);

    // ## Enforce: no authority may have been granted
    //
    // Checked before the balance test because it is the cheaper rejection and
    // the more dangerous condition - an authority grant costs nothing now and
    // everything later, so a zero delta here is not reassuring.
    let authority_after = authority_fingerprint(vault_token_account)?;
    if authority_after != authority_before {
        msg!("REJECTED: instruction altered an authority field on the vault account");
        return Err(ProgramError::Custom(2));
    }

    // ## Enforce: no other vault holding may leave
    //
    // The delegation covers one mint. Everything else the vault owns must come
    // back untouched - it may grow, it may not shrink. This is what closes the
    // unmetered-mint hole, and it needs no knowledge of which mints exist.
    for (a, sibling_before, sibling_fp) in siblings.iter() {
        if authority_fingerprint(a)? != *sibling_fp {
            msg!("REJECTED: instruction altered an authority field on another vault account");
            return Err(ProgramError::Custom(2));
        }
        let sibling_after = token_amount(a)?;
        if sibling_after < *sibling_before {
            msg!(
                "REJECTED: unmetered vault account fell {} -> {}",
                sibling_before,
                sibling_after
            );
            return Err(ProgramError::Custom(3));
        }
    }

    // ## Enforce: the spend is within cap
    let spent = before.saturating_sub(after);
    msg!("spent: {} (cap {})", spent, cap);
    if spent > cap {
        msg!("REJECTED: spend {} exceeds cap {}", spent, cap);
        return Err(ProgramError::Custom(1));
    }

    Ok(())
}
