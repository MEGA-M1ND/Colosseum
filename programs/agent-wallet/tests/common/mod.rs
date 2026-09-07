//! ## Shared test fixture
//!
//! Runs the real SPL Token program and the Anchor program natively under
//! `solana-program-test` - no validator, no SBF toolchain.

#![allow(dead_code)]

use agent_wallet::errors::AgentWalletError;
use agent_wallet::state::{Delegation, PolicyArgs, Vault};
use agent_wallet::{accounts as acc, instruction as ixn};
use anchor_lang::solana_program::entrypoint::ProgramResult;
use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_program_test::{processor, BanksClientError, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account,
    account_info::AccountInfo,
    clock::Clock,
    instruction::{AccountMeta, Instruction, InstructionError},
    transaction::TransactionError,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

/// Anchor's generated `entry` ties the slice reference and the `AccountInfo`
/// lifetimes together; `processor!` wants them independent. This re-links only
/// lifetimes, no types, and the accounts outlive the call. Test scaffolding -
/// never reachable from the deployed program.
fn entry_shim(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let accounts: &[AccountInfo] = unsafe { core::mem::transmute(accounts) };
    agent_wallet::entry(program_id, accounts, data)
}

pub const START_BALANCE: u64 = 1_000;
pub const DECIMALS: u8 = 6;
pub const SYSTEM_PROGRAM: Pubkey = anchor_lang::solana_program::system_program::ID;

// ## SPL token encoding

pub fn token_account_data(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut d = vec![0u8; 165];
    d[0..32].copy_from_slice(mint.as_ref());
    d[32..64].copy_from_slice(owner.as_ref());
    d[64..72].copy_from_slice(&amount.to_le_bytes());
    d[108] = 1; // Initialized
    d
}

pub fn mint_data(decimals: u8, supply: u64) -> Vec<u8> {
    let mut d = vec![0u8; 82];
    d[36..44].copy_from_slice(&supply.to_le_bytes());
    d[44] = decimals;
    d[45] = 1;
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

fn funded(lamports: u64) -> Account {
    Account {
        lamports,
        data: vec![],
        owner: SYSTEM_PROGRAM,
        executable: false,
        rent_epoch: 0,
    }
}

// ## Fixture

pub struct Fix {
    pub owner: Keypair,
    pub agent: Keypair,
    pub mint: Pubkey,
    pub mint_b: Pubkey,
    pub vault: Pubkey,
    pub delegation: Pubkey,
    pub vault_token: Pubkey,
    pub vault_token_b: Pubkey,
    pub dest_token: Pubkey,
    pub dest_token_b: Pubkey,
    pub dest_owner: Pubkey,
}

pub fn setup() -> (ProgramTest, Fix) {
    let mut pt = ProgramTest::new("agent_wallet", agent_wallet::ID, processor!(entry_shim));
    pt.add_program(
        "spl_token",
        spl_token::id(),
        processor!(spl_token::processor::Processor::process),
    );

    let owner = Keypair::new();
    let agent = Keypair::new();
    let mint = Pubkey::new_unique();
    let mint_b = Pubkey::new_unique();
    let dest_owner = Pubkey::new_unique();

    let (vault, _) =
        Pubkey::find_program_address(&[Vault::SEED, owner.pubkey().as_ref()], &agent_wallet::ID);
    let (delegation, _) = Pubkey::find_program_address(
        &[Delegation::SEED, vault.as_ref(), agent.pubkey().as_ref()],
        &agent_wallet::ID,
    );

    let vault_token = Pubkey::new_unique();
    let vault_token_b = Pubkey::new_unique();
    let dest_token = Pubkey::new_unique();
    let dest_token_b = Pubkey::new_unique();

    pt.add_account(owner.pubkey(), funded(1_000_000_000));
    pt.add_account(agent.pubkey(), funded(1_000_000_000));
    pt.add_account(mint, spl_account(mint_data(DECIMALS, START_BALANCE)));
    pt.add_account(mint_b, spl_account(mint_data(DECIMALS, START_BALANCE)));
    pt.add_account(
        vault_token,
        spl_account(token_account_data(&mint, &vault, START_BALANCE)),
    );
    pt.add_account(
        vault_token_b,
        spl_account(token_account_data(&mint_b, &vault, START_BALANCE)),
    );
    pt.add_account(
        dest_token,
        spl_account(token_account_data(&mint, &dest_owner, 0)),
    );
    pt.add_account(
        dest_token_b,
        spl_account(token_account_data(&mint_b, &dest_owner, 0)),
    );

    (
        pt,
        Fix {
            owner,
            agent,
            mint,
            mint_b,
            vault,
            delegation,
            vault_token,
            vault_token_b,
            dest_token,
            dest_token_b,
            dest_owner,
        },
    )
}

// ## Transaction helpers

pub async fn send(
    ctx: &mut ProgramTestContext,
    ixs: &[Instruction],
    signers: &[&Keypair],
) -> Result<(), BanksClientError> {
    // A fresh blockhash per send. Two identical transactions under one blockhash
    // share a signature, and the runtime then dedupes the second - it silently
    // returns the first one's result instead of executing. That turns a repeated
    // spend into a no-op and a retried rejection into a false pass.
    let blockhash = ctx
        .get_new_latest_blockhash()
        .await
        .expect("new blockhash");
    let mut all = vec![&ctx.payer];
    all.extend_from_slice(signers);
    let tx = Transaction::new_signed_with_payer(ixs, Some(&ctx.payer.pubkey()), &all, blockhash);
    ctx.banks_client.process_transaction(tx).await
}

pub fn ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction {
        program_id: agent_wallet::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

pub async fn balance(ctx: &mut ProgramTestContext, key: Pubkey) -> u64 {
    let a = ctx.banks_client.get_account(key).await.unwrap().unwrap();
    u64::from_le_bytes(a.data[64..72].try_into().unwrap())
}

pub async fn delegation_state(ctx: &mut ProgramTestContext, key: Pubkey) -> Delegation {
    let a = ctx.banks_client.get_account(key).await.unwrap().unwrap();
    Delegation::try_deserialize(&mut a.data.as_slice()).unwrap()
}


// ## Instruction builders
//
// One place that knows each instruction's account list, so a test reads as the
// scenario it is testing rather than as account plumbing.

pub fn default_policy() -> PolicyArgs {
    PolicyArgs {
        per_tx_limit: 100,
        total_limit: 500,
        window_duration: 0,
        window_limit: 0,
        expires_at: i64::MAX,
        allowed_destinations: vec![],
        allowed_programs: vec![],
    }
}

pub async fn init_vault(ctx: &mut ProgramTestContext, f: &Fix) -> Result<(), BanksClientError> {
    let i = ix(
        acc::InitializeVault {
            owner: f.owner.pubkey(),
            vault: f.vault,
            system_program: SYSTEM_PROGRAM,
        },
        ixn::InitializeVault {},
    );
    send(ctx, &[i], &[&f.owner]).await
}

pub async fn delegate(
    ctx: &mut ProgramTestContext,
    f: &Fix,
    policy: PolicyArgs,
) -> Result<(), BanksClientError> {
    let i = ix(
        acc::CreateDelegation {
            owner: f.owner.pubkey(),
            vault: f.vault,
            agent: f.agent.pubkey(),
            mint: f.mint,
            delegation: f.delegation,
            system_program: SYSTEM_PROGRAM,
        },
        ixn::CreateDelegation { policy },
    );
    send(ctx, &[i], &[&f.owner]).await
}

/// Vault + delegation in one step, for tests whose subject is the spend.
pub async fn bootstrap(ctx: &mut ProgramTestContext, f: &Fix, policy: PolicyArgs) {
    init_vault(ctx, f).await.unwrap();
    delegate(ctx, f, policy).await.unwrap();
}

pub fn transfer_ix(f: &Fix, amount: u64) -> Instruction {
    ix(
        acc::AgentTransfer {
            agent: f.agent.pubkey(),
            vault: f.vault,
            delegation: f.delegation,
            mint: f.mint,
            vault_token_account: f.vault_token,
            destination_token_account: f.dest_token,
            token_program: spl_token::id(),
        },
        ixn::AgentTransfer { amount },
    )
}

pub async fn agent_spend(
    ctx: &mut ProgramTestContext,
    f: &Fix,
    amount: u64,
) -> Result<(), BanksClientError> {
    let i = transfer_ix(f, amount);
    send(ctx, &[i], &[&f.agent]).await
}

pub fn invoke_ix(
    f: &Fix,
    target: Pubkey,
    data: Vec<u8>,
    remaining: Vec<AccountMeta>,
) -> Instruction {
    let mut metas = acc::AgentInvoke {
        agent: f.agent.pubkey(),
        vault: f.vault,
        delegation: f.delegation,
        mint: f.mint,
        vault_token_account: f.vault_token,
        target_program: target,
    }
    .to_account_metas(None);
    metas.extend(remaining);
    Instruction {
        program_id: agent_wallet::ID,
        accounts: metas,
        data: ixn::AgentInvoke { data }.data(),
    }
}

pub async fn agent_invoke(
    ctx: &mut ProgramTestContext,
    f: &Fix,
    target: Pubkey,
    data: Vec<u8>,
    remaining: Vec<AccountMeta>,
) -> Result<(), BanksClientError> {
    let i = invoke_ix(f, target, data, remaining);
    send(ctx, &[i], &[&f.agent]).await
}

// ## SPL token instruction data

pub fn spl_transfer(amount: u64) -> Vec<u8> {
    let mut d = vec![3u8];
    d.extend_from_slice(&amount.to_le_bytes());
    d
}

pub fn spl_approve(amount: u64) -> Vec<u8> {
    let mut d = vec![4u8];
    d.extend_from_slice(&amount.to_le_bytes());
    d
}

// ## Assertions

/// Assert the transaction failed with exactly this program error - not merely
/// that it failed. A test that only checks `is_err()` passes when the
/// instruction is malformed and the guard never ran.
pub fn assert_err(res: Result<(), BanksClientError>, expected: AgentWalletError) {
    let err = res.expect_err("expected this to be rejected, but it succeeded");
    let code = match err {
        BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(c),
        )) => c,
        other => panic!("expected a custom program error, got: {other:?}"),
    };
    let want = expected as u32 + anchor_lang::error::ERROR_CODE_OFFSET;
    assert_eq!(
        code, want,
        "rejected for the wrong reason: got code {code}, expected {want} ({expected:?})"
    );
}

// ## Clock

pub async fn advance_clock(ctx: &mut ProgramTestContext, seconds: i64) {
    let mut clock: Clock = ctx.banks_client.get_sysvar().await.unwrap();
    clock.unix_timestamp += seconds;
    ctx.set_sysvar(&clock);
}

pub async fn now(ctx: &mut ProgramTestContext) -> i64 {
    let clock: Clock = ctx.banks_client.get_sysvar().await.unwrap();
    clock.unix_timestamp
}

fn custom_code(res: Result<(), BanksClientError>) -> u32 {
    match res.expect_err("expected this to be rejected, but it succeeded") {
        BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(c),
        )) => c,
        other => panic!("expected a custom program error, got: {other:?}"),
    }
}

/// Anchor's own account-constraint violations live in 2000-2999. Used where the
/// rejection comes from a `seeds`/`has_one` constraint rather than our policy.
pub fn assert_constraint_err(res: Result<(), BanksClientError>) {
    let code = custom_code(res);
    assert!(
        (2000..3000).contains(&code),
        "expected an Anchor constraint violation (2000-2999), got {code}"
    );
}
