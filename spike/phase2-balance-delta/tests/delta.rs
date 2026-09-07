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

/// COption discriminant for the delegate field: 0 = None, 1 = Some.
async fn delegate_tag(bc: &mut solana_program_test::BanksClient, key: Pubkey) -> u32 {
    let acct = bc.get_account(key).await.unwrap().unwrap();
    u32::from_le_bytes(acct.data[72..76].try_into().unwrap())
}

/// COption discriminant for the close_authority field.
async fn close_authority_tag(bc: &mut solana_program_test::BanksClient, key: Pubkey) -> u32 {
    let acct = bc.get_account(key).await.unwrap().unwrap();
    u32::from_le_bytes(acct.data[129..133].try_into().unwrap())
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

// ## C. The approve hole - now closed
//
// `approve` moves no tokens, so the balance delta is zero. Before the fix the
// guard allowed it and the delegate drained the vault afterwards. The authority
// fingerprint catches it: the delegate field changed, so the CPI is rejected
// and reverted.

#[tokio::test]
async fn approve_is_rejected() {
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
    let res = ctx.banks_client.process_transaction(tx).await;

    assert!(
        res.is_err(),
        "approve grants standing authority at zero delta and must be rejected"
    );
    assert_eq!(
        delegate_tag(&mut ctx.banks_client, f.vault_token).await,
        0,
        "delegation must be reverted, not merely flagged"
    );
    assert_eq!(
        balance(&mut ctx.banks_client, f.vault_token).await,
        START_BALANCE
    );

    // And the attacker has no delegation to spend against.
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
    assert!(
        ctx.banks_client.process_transaction(tx).await.is_err(),
        "the follow-up drain must fail - there is no delegation"
    );
    assert_eq!(
        balance(&mut ctx.banks_client, f.vault_token).await,
        START_BALANCE,
        "vault intact"
    );
}

// ## C2. SetAuthority is the same class of attack
//
// Handing over close authority (or ownership) also costs zero tokens now. The
// same fingerprint covers it without the guard knowing what SetAuthority is.

#[tokio::test]
async fn set_authority_is_rejected() {
    let (pt, f) = fixture();
    let mut ctx = pt.start_with_context().await;

    let attacker = Keypair::new();

    // spl-token SetAuthority = tag 6; AuthorityType::CloseAccount = 3;
    // new_authority is packed as a 1-byte Some/None tag then the pubkey.
    let mut inner = vec![6u8, 3u8, 1u8];
    inner.extend_from_slice(attacker.pubkey().as_ref());

    let ix = guarded(
        &f,
        100,
        inner,
        vec![
            AccountMeta::new(f.vault_token, false),
            AccountMeta::new_readonly(f.vault_authority, false),
        ],
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    assert!(
        ctx.banks_client.process_transaction(tx).await.is_err(),
        "granting close authority must be rejected"
    );
    assert_eq!(
        close_authority_tag(&mut ctx.banks_client, f.vault_token).await,
        0,
        "close authority must be unset after the revert"
    );
}

// ## Two-mint fixture
//
// One vault authority holding two different mints, so a delegation scoped to
// mint A can be tested against the vault's mint B holdings.

struct TwoMint {
    vault_authority: Pubkey,
    vault_a: Pubkey,
    dest_a: Pubkey,
    vault_b: Pubkey,
    dest_b: Pubkey,
}

fn two_mint_fixture() -> (ProgramTest, TwoMint) {
    let mut pt = ProgramTest::new("spike", GUARD_ID, processor!(spike::process_instruction));
    pt.add_program(
        "spl_token",
        spl_token::id(),
        processor!(spl_token::processor::Processor::process),
    );

    let (vault_authority, _) = Pubkey::find_program_address(&[spike::VAULT_SEED], &GUARD_ID);
    let mint_a = Pubkey::new_unique();
    let mint_b = Pubkey::new_unique();
    let vault_a = Pubkey::new_unique();
    let vault_b = Pubkey::new_unique();
    let dest_a = Pubkey::new_unique();
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
        dest_a,
        spl_account(token_account_data(&mint_a, &dest_owner, 0)),
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

    (
        pt,
        TwoMint {
            vault_authority,
            vault_a,
            dest_a,
            vault_b,
            dest_b,
        },
    )
}

// ## D. The unmetered-mint hole - now closed
//
// The guard is pointed at the vault's mint-A account; the instruction moves
// mint B. Before the fix the delta on A was zero and it sailed through. Now
// every vault-owned token account reachable by the CPI is snapshotted, and any
// that shrinks is a rejection.

#[tokio::test]
async fn unmetered_mint_is_rejected() {
    let (pt, f) = two_mint_fixture();
    let mut ctx = pt.start_with_context().await;

    let mut inner = vec![3u8];
    inner.extend_from_slice(&START_BALANCE.to_le_bytes());

    let mut data = 100u64.to_le_bytes().to_vec(); // cap of 100
    data.extend_from_slice(&inner);

    let ix = Instruction {
        program_id: GUARD_ID,
        accounts: vec![
            AccountMeta::new_readonly(f.vault_authority, false),
            AccountMeta::new(f.vault_a, false), // <- the metered account
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(f.vault_b, false), // <- what the instruction moves
            AccountMeta::new(f.dest_b, false),
            AccountMeta::new_readonly(f.vault_authority, false),
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    assert!(
        ctx.banks_client.process_transaction(tx).await.is_err(),
        "draining an unmetered vault holding must be rejected"
    );
    assert_eq!(
        balance(&mut ctx.banks_client, f.vault_b).await,
        START_BALANCE,
        "mint B must be intact"
    );
    assert_eq!(balance(&mut ctx.banks_client, f.vault_a).await, START_BALANCE);
    assert_eq!(balance(&mut ctx.banks_client, f.dest_b).await, 0);
}

// ## E. The fix must not over-reject
//
// A guard that refuses everything is not a guard. A vault account merely being
// present in the account list is fine - only a decrease is a rejection.

#[tokio::test]
async fn untouched_sibling_does_not_block_a_valid_spend() {
    let (pt, f) = two_mint_fixture();
    let mut ctx = pt.start_with_context().await;

    // Transfer 50 of mint A, within the cap, while mint B rides along unused.
    let mut inner = vec![3u8];
    inner.extend_from_slice(&50u64.to_le_bytes());

    let mut data = 100u64.to_le_bytes().to_vec();
    data.extend_from_slice(&inner);

    let ix = Instruction {
        program_id: GUARD_ID,
        accounts: vec![
            AccountMeta::new_readonly(f.vault_authority, false),
            AccountMeta::new(f.vault_a, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            // spl-token Transfer reads the first three; vault_b is a trailing
            // extra the guard still puts in scope.
            AccountMeta::new(f.vault_a, false),
            AccountMeta::new(f.dest_a, false),
            AccountMeta::new_readonly(f.vault_authority, false),
            AccountMeta::new(f.vault_b, false),
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();

    assert_eq!(balance(&mut ctx.banks_client, f.vault_a).await, 950);
    assert_eq!(balance(&mut ctx.banks_client, f.dest_a).await, 50);
    assert_eq!(
        balance(&mut ctx.banks_client, f.vault_b).await,
        START_BALANCE,
        "untouched sibling stays untouched"
    );
}

// ## F. The native SOL hole
//
// The vault PDA holds lamports of its own. A system transfer signed by the PDA
// moves zero tokens, alters no authority field, and touches no sibling token
// account - so every check so far is satisfied while the vault's SOL leaves.

#[tokio::test]
async fn sol_drain_is_rejected() {
    let (pt, f) = fixture();
    let mut pt = pt;
    let attacker = Pubkey::new_unique();
    pt.add_account(
        attacker,
        Account {
            lamports: 1_000_000,
            data: vec![],
            owner: solana_sdk::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );
    let mut ctx = pt.start_with_context().await;

    let before = ctx
        .banks_client
        .get_account(f.vault_authority)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // SystemInstruction::Transfer = variant 2 (u32 LE), then lamports (u64 LE).
    let mut inner = 2u32.to_le_bytes().to_vec();
    inner.extend_from_slice(&5_000_000u64.to_le_bytes());

    let mut data = 100u64.to_le_bytes().to_vec(); // token cap, irrelevant here
    data.extend_from_slice(&inner);

    let ix = Instruction {
        program_id: GUARD_ID,
        accounts: vec![
            AccountMeta::new(f.vault_authority, false), // writable: SOL leaves it
            AccountMeta::new(f.vault_token, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new(f.vault_authority, false),
            AccountMeta::new(attacker, false),
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

    assert!(res.is_err(), "draining the vault's SOL must be rejected");

    let after = ctx
        .banks_client
        .get_account(f.vault_authority)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_eq!(after, before, "vault lamports must be intact");
}
