//! ## Phase 2 - arbitrary CPI, bounded by measurement
//!
//! The attacks here are the ones the spike found. Each was a working exploit
//! before the guard existed; each is kept as a rejection with its own error
//! code so a pass cannot come from a malformed instruction.

mod common;

use agent_wallet::errors::AgentWalletError as E;
use agent_wallet::state::PolicyArgs;
use common::*;
use solana_sdk::{instruction::AccountMeta, pubkey::Pubkey, signature::Signer};

fn invoke_policy() -> PolicyArgs {
    PolicyArgs {
        allowed_programs: vec![spl_token::id()],
        ..default_policy()
    }
}

/// [source, destination, authority] - the shape spl-token Transfer expects.
fn transfer_accounts(f: &Fix) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(f.vault_token, false),
        AccountMeta::new(f.dest_token, false),
        AccountMeta::new_readonly(f.vault, false),
    ]
}

#[tokio::test]
async fn within_budget_is_allowed() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, invoke_policy()).await;

    agent_invoke(
        &mut ctx,
        &f,
        spl_token::id(),
        spl_transfer(50),
        transfer_accounts(&f),
    )
    .await
    .unwrap();

    assert_eq!(balance(&mut ctx, f.vault_token).await, 950);
    assert_eq!(balance(&mut ctx, f.dest_token).await, 50);
    assert_eq!(
        delegation_state(&mut ctx, f.delegation).await.total_spent,
        50,
        "the delta is what charges the budget"
    );
}

#[tokio::test]
async fn over_budget_is_rejected_and_reverted() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, invoke_policy()).await;

    assert_err(
        agent_invoke(
            &mut ctx,
            &f,
            spl_token::id(),
            spl_transfer(500), // per-tx cap is 100
            transfer_accounts(&f),
        )
        .await,
        E::ExceedsPerTxLimit,
    );

    // The CPI moved the tokens before the check ran; the Err must undo it.
    assert_eq!(balance(&mut ctx, f.vault_token).await, START_BALANCE);
    assert_eq!(balance(&mut ctx, f.dest_token).await, 0);
}

#[tokio::test]
async fn approve_is_rejected() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, invoke_policy()).await;

    let attacker = Pubkey::new_unique();

    // Approve moves no tokens, so the balance delta is zero. Only the authority
    // fingerprint catches it.
    assert_err(
        agent_invoke(
            &mut ctx,
            &f,
            spl_token::id(),
            spl_approve(START_BALANCE),
            vec![
                AccountMeta::new(f.vault_token, false),
                AccountMeta::new_readonly(attacker, false),
                AccountMeta::new_readonly(f.vault, false),
            ],
        )
        .await,
        E::AuthorityAltered,
    );

    // Reverted, not merely flagged: no delegate is set on the account.
    let acct = ctx
        .banks_client
        .get_account(f.vault_token)
        .await
        .unwrap()
        .unwrap();
    let delegate_tag = u32::from_le_bytes(acct.data[72..76].try_into().unwrap());
    assert_eq!(delegate_tag, 0, "delegation must be reverted");
}

#[tokio::test]
async fn draining_an_unmetered_mint_is_rejected() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, invoke_policy()).await;

    // The delegation covers mint A. This moves mint B out of the same vault -
    // zero delta on the metered account.
    assert_err(
        agent_invoke(
            &mut ctx,
            &f,
            spl_token::id(),
            spl_transfer(START_BALANCE),
            vec![
                AccountMeta::new(f.vault_token_b, false),
                AccountMeta::new(f.dest_token_b, false),
                AccountMeta::new_readonly(f.vault, false),
            ],
        )
        .await,
        E::UnmeteredHoldingFell,
    );

    assert_eq!(balance(&mut ctx, f.vault_token_b).await, START_BALANCE);
    assert_eq!(balance(&mut ctx, f.dest_token_b).await, 0);
}

#[tokio::test]
async fn program_must_be_allowlisted() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    // Default policy has an EMPTY program allowlist: invoke is opt-in.
    bootstrap(&mut ctx, &f, default_policy()).await;

    assert_err(
        agent_invoke(
            &mut ctx,
            &f,
            spl_token::id(),
            spl_transfer(10),
            transfer_accounts(&f),
        )
        .await,
        E::ProgramNotAllowed,
    );
}

#[tokio::test]
async fn cannot_invoke_this_program() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    let policy = PolicyArgs {
        allowed_programs: vec![agent_wallet::ID],
        ..default_policy()
    };
    bootstrap(&mut ctx, &f, policy).await;

    // Even explicitly allowlisted, re-entrancy into ourselves is refused.
    assert_err(
        agent_invoke(
            &mut ctx,
            &f,
            agent_wallet::ID,
            vec![0u8; 8],
            transfer_accounts(&f),
        )
        .await,
        E::SelfInvokeForbidden,
    );
}
