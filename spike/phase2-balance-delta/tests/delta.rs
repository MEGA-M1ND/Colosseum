//! ## Phase 2 spike tests
//!
//! Three questions:
//!   A. Does a within-budget arbitrary CPI go through?
//!   B. Does an over-budget one get rejected, and does the value come back?
//!   C. Does an `approve` slip past a balance-delta check? (the suspected hole)

use solana_program_test::{processor, ProgramTest};
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

// ## SPL token account/mint encoding helpers

fn token_account_data(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut d = vec![0u8; 165];
    d[0..32].copy_from_slice(mint.as_ref());
    d[32..64].copy_from_slice(owner.as_ref());
    d[64..72].copy_from_slice(&amount.to_le_bytes());
    d[108] = 1; // state = Initialized
    d
}

fn mint_data(decimals: u8, supply: u64) -> Vec<u8> {
    let mut d = vec![0u8; 82];
    d[36..44].copy_from_slice(&supply.to_le_bytes());
    d[44] = decimals;
    d[45] = 1; // is_initialized
    d
}

fn spl_account(data: Vec<u8>) -> Account {
    Account {
        lamports: 10_000_000,
        data,
        owner: spl_token::id(),
        executable: false,
        rent_epoch: 0,
    }
}

// ## Fixture

const GUARD_ID: Pubkey = Pubkey::new_from_array([7u8; 32]);
const START_BALANCE: u64 = 1_000;

struct Fixture {
    vault_authority: Pubkey,
    vault_token: Pubkey,
    dest_token: Pubkey,
}

fn fixture() -> (ProgramTest, Fixture) {
    let mut pt = ProgramTest::new("spike", GUARD_ID, processor!(spike::process_instruction));
    pt.add_program(
        "spl_token",
        spl_token::id(),
        processor!(spl_token::processor::Processor::process),
    );

    let (vault_authority, _) = Pubkey::find_program_address(&[spike::VAULT_SEED], &GUARD_ID);
    let mint = Pubkey::new_unique();
    let vault_token = Pubkey::new_unique();
    let dest_token = Pubkey::new_unique();
    let dest_owner = Pubkey::new_unique();

    pt.add_account(mint, spl_account(mint_data(6, START_BALANCE)));
    pt.add_account(
        vault_token,
        spl_account(token_account_data(&mint, &vault_authority, START_BALANCE)),
    );
    pt.add_account(
        dest_token,
        spl_account(token_account_data(&mint, &dest_owner, 0)),
    );
    // The PDA itself: system-owned, no data. It only ever signs.
    pt.add_account(
        vault_authority,
        Account {
            lamports: 10_000_000,
            data: vec![],
            owner: solana_sdk::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    (
        pt,
        Fixture {
            vault_authority,
            vault_token,
            dest_token,
        },
    )
}

// ## Build a guard instruction wrapping an inner spl-token instruction

fn guarded(
    f: &Fixture,
    cap: u64,
    inner_data: Vec<u8>,
    inner_accounts: Vec<AccountMeta>,
) -> Instruction {
    let mut data = cap.to_le_bytes().to_vec();
    data.extend_from_slice(&inner_data);

    let mut accounts = vec![
        AccountMeta::new_readonly(f.vault_authority, false),
        AccountMeta::new(f.vault_token, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    accounts.extend(inner_accounts);

    Instruction {
        program_id: GUARD_ID,
        accounts,
        data,
    }
}

async fn balance(bc: &mut solana_program_test::BanksClient, key: Pubkey) -> u64 {
    let acct = bc.get_account(key).await.unwrap().unwrap();
    u64::from_le_bytes(acct.data[64..72].try_into().unwrap())
}

// ## A. Within budget -> allowed

#[tokio::test]
async fn within_budget_is_allowed() {
    let (pt, f) = fixture();
    let mut ctx = pt.start_with_context().await;

    // spl-token Transfer = tag 3
    let mut inner = vec![3u8];
    inner.extend_from_slice(&50u64.to_le_bytes());

    let ix = guarded(
        &f,
        100, // cap
        inner,
        vec![
            AccountMeta::new(f.vault_token, false),
            AccountMeta::new(f.dest_token, false),
            AccountMeta::new_readonly(f.vault_authority, false),
        ],
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();

    assert_eq!(balance(&mut ctx.banks_client, f.vault_token).await, 950);
    assert_eq!(balance(&mut ctx.banks_client, f.dest_token).await, 50);
}

// ## B. Over budget -> rejected, and the transfer is reverted

#[tokio::test]
async fn over_budget_is_rejected_and_reverted() {
    let (pt, f) = fixture();
    let mut ctx = pt.start_with_context().await;

    let mut inner = vec![3u8];
    inner.extend_from_slice(&500u64.to_le_bytes()); // way over

    let ix = guarded(
        &f,
        100,
        inner,
        vec![
            AccountMeta::new(f.vault_token, false),
            AccountMeta::new(f.dest_token, false),
            AccountMeta::new_readonly(f.vault_authority, false),
        ],
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    let err = ctx.banks_client.process_transaction(tx).await;
    assert!(err.is_err(), "over-budget spend should be rejected");

    // The CPI already moved tokens before the check ran. The Err must undo it.
    assert_eq!(
        balance(&mut ctx.banks_client, f.vault_token).await,
        START_BALANCE,
        "vault must be untouched after rejection"
    );
    assert_eq!(balance(&mut ctx.banks_client, f.dest_token).await, 0);
}

// ## C. The approve hole
//
// `approve` moves no tokens, so the delta is zero and the guard allows it.
// The delegate then drains the vault directly, never touching the guard.

#[tokio::test]
async fn approve_slips_past_the_delta_check() {
    let (pt, f) = fixture();
    let mut ctx = pt.start_with_context().await;

    let attacker = Keypair::new();

    // spl-token Approve = tag 4
    let mut inner = vec![4u8];
    inner.extend_from_slice(&START_BALANCE.to_le_bytes());

    let ix = guarded(
        &f,
        100, // a 100-token cap on a vault of 1000
        inner,
        vec![
            AccountMeta::new(f.vault_token, false),
            AccountMeta::new_readonly(attacker.pubkey(), false),
            AccountMeta::new_readonly(f.vault_authority, false),
        ],
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    let approved = ctx.banks_client.process_transaction(tx).await;

    assert!(
        approved.is_ok(),
        "approve costs nothing now, so the delta check lets it through"
    );
    assert_eq!(
        balance(&mut ctx.banks_client, f.vault_token).await,
        START_BALANCE,
        "no tokens moved yet - which is exactly why it passed"
    );

    // Now the delegate spends the full balance, outside the guard entirely.
    let mut drain = vec![3u8];
    drain.extend_from_slice(&START_BALANCE.to_le_bytes());
    let drain_ix = Instruction {
        program_id: spl_token::id(),
        accounts: vec![
            AccountMeta::new(f.vault_token, false),
            AccountMeta::new(f.dest_token, false),
            AccountMeta::new_readonly(attacker.pubkey(), true),
        ],
        data: drain,
    };

    let tx = Transaction::new_signed_with_payer(
        &[drain_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &attacker],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();

    assert_eq!(
        balance(&mut ctx.banks_client, f.vault_token).await,
        0,
        "vault drained through a delegation the guard never metered"
    );
}

// ## D. The unmetered-mint hole
//
// The guard meters exactly one token account. Give the vault a second mint and
// the agent moves it out freely - the delta on the metered account is zero.
// This is why the design constrains a delegation to a single mint.

#[tokio::test]
async fn unmetered_mint_drains_freely() {
    let mut pt = ProgramTest::new("spike", GUARD_ID, processor!(spike::process_instruction));
    pt.add_program(
        "spl_token",
        spl_token::id(),
        processor!(spl_token::processor::Processor::process),
    );

    let (vault_authority, _) = Pubkey::find_program_address(&[spike::VAULT_SEED], &GUARD_ID);

    // Mint A - the one the guard watches.
    let mint_a = Pubkey::new_unique();
    let vault_a = Pubkey::new_unique();
    // Mint B - same vault, not watched.
    let mint_b = Pubkey::new_unique();
    let vault_b = Pubkey::new_unique();
    let dest_b = Pubkey::new_unique();
    let dest_owner = Pubkey::new_unique();

    pt.add_account(mint_a, spl_account(mint_data(6, START_BALANCE)));
    pt.add_account(mint_b, spl_account(mint_data(6, START_BALANCE)));
    pt.add_account(
        vault_a,
        spl_account(token_account_data(&mint_a, &vault_authority, START_BALANCE)),
    );
    pt.add_account(
        vault_b,
        spl_account(token_account_data(&mint_b, &vault_authority, START_BALANCE)),
    );
    pt.add_account(
        dest_b,
        spl_account(token_account_data(&mint_b, &dest_owner, 0)),
    );
    pt.add_account(
        vault_authority,
        Account {
            lamports: 10_000_000,
            data: vec![],
            owner: solana_sdk::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    let mut ctx = pt.start_with_context().await;

    // Meter vault_a, but move everything out of vault_b.
    let mut inner = vec![3u8];
    inner.extend_from_slice(&START_BALANCE.to_le_bytes());

    let mut data = 100u64.to_le_bytes().to_vec(); // cap of 100
    data.extend_from_slice(&inner);

    let ix = Instruction {
        program_id: GUARD_ID,
        accounts: vec![
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new(vault_a, false), // <- the metered account
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(vault_b, false), // <- what actually moves
            AccountMeta::new(dest_b, false),
            AccountMeta::new_readonly(vault_authority, false),
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    let res = ctx.banks_client.process_transaction(tx).await;

    assert!(
        res.is_ok(),
        "guard sees no change on the metered account, so it allows it"
    );
    assert_eq!(balance(&mut ctx.banks_client, vault_a).await, START_BALANCE);
    assert_eq!(
        balance(&mut ctx.banks_client, vault_b).await,
        0,
        "unmetered mint drained past a 100-token cap"
    );
}
