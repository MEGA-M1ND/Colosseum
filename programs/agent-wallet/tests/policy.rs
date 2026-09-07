//! ## Policy enforcement
//!
//! Every limit, proven to reject - and each rejection checked by error code, so
//! a test cannot pass because the instruction was malformed.

mod common;

use agent_wallet::errors::AgentWalletError as E;
use agent_wallet::state::PolicyArgs;
use agent_wallet::{accounts as acc, instruction as ixn};
use common::*;
use solana_sdk::signature::Signer;

#[tokio::test]
async fn per_tx_limit_rejects() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, default_policy()).await;

    assert_err(agent_spend(&mut ctx, &f, 101).await, E::ExceedsPerTxLimit);
    assert_eq!(balance(&mut ctx, f.vault_token).await, START_BALANCE);

    // The cap itself is allowed - an off-by-one here would be silent.
    agent_spend(&mut ctx, &f, 100).await.unwrap();
    assert_eq!(balance(&mut ctx, f.vault_token).await, 900);
}

#[tokio::test]
async fn total_limit_rejects_cumulatively() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, default_policy()).await; // total 500, per-tx 100

    for _ in 0..5 {
        agent_spend(&mut ctx, &f, 100).await.unwrap();
    }
    assert_eq!(delegation_state(&mut ctx, f.delegation).await.total_spent, 500);

    assert_err(agent_spend(&mut ctx, &f, 1).await, E::ExceedsTotalLimit);
    assert_eq!(balance(&mut ctx, f.vault_token).await, 500);
}

#[tokio::test]
async fn window_limit_rejects_then_rolls_over() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    let policy = PolicyArgs {
        window_duration: 3600,
        window_limit: 150,
        ..default_policy()
    };
    bootstrap(&mut ctx, &f, policy).await;

    agent_spend(&mut ctx, &f, 100).await.unwrap();
    assert_err(agent_spend(&mut ctx, &f, 100).await, E::ExceedsWindowLimit);

    // Tumbling window: once it expires the counter resets wholesale.
    advance_clock(&mut ctx, 3601).await;
    agent_spend(&mut ctx, &f, 100).await.unwrap();

    let d = delegation_state(&mut ctx, f.delegation).await;
    assert_eq!(d.total_spent, 200, "lifetime total keeps accumulating");
    assert_eq!(d.window_spent, 100, "window counter restarted");
}

#[tokio::test]
async fn expiry_rejects() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    let expires_at = now(&mut ctx).await + 60;
    bootstrap(
        &mut ctx,
        &f,
        PolicyArgs {
            expires_at,
            ..default_policy()
        },
    )
    .await;

    agent_spend(&mut ctx, &f, 10).await.unwrap();
    advance_clock(&mut ctx, 120).await;
    assert_err(agent_spend(&mut ctx, &f, 10).await, E::DelegationExpired);
}

#[tokio::test]
async fn revocation_is_immediate() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, default_policy()).await;

    agent_spend(&mut ctx, &f, 10).await.unwrap();

    let revoke = ix(
        acc::ModifyDelegation {
            owner: f.owner.pubkey(),
            vault: f.vault,
            delegation: f.delegation,
        },
        ixn::RevokeDelegation {},
    );
    send(&mut ctx, &[revoke], &[&f.owner]).await.unwrap();

    assert_err(agent_spend(&mut ctx, &f, 10).await, E::DelegationRevoked);
}

#[tokio::test]
async fn pause_stops_every_agent() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, default_policy()).await;

    let pause = ix(
        acc::OwnerOnly {
            owner: f.owner.pubkey(),
            vault: f.vault,
        },
        ixn::SetPaused { paused: true },
    );
    send(&mut ctx, &[pause], &[&f.owner]).await.unwrap();
    assert_err(agent_spend(&mut ctx, &f, 10).await, E::VaultPaused);

    let unpause = ix(
        acc::OwnerOnly {
            owner: f.owner.pubkey(),
            vault: f.vault,
        },
        ixn::SetPaused { paused: false },
    );
    send(&mut ctx, &[unpause], &[&f.owner]).await.unwrap();
    agent_spend(&mut ctx, &f, 10).await.unwrap();
}

#[tokio::test]
async fn destination_allowlist_rejects_strangers() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    let policy = PolicyArgs {
        allowed_destinations: vec![solana_sdk::pubkey::Pubkey::new_unique()],
        ..default_policy()
    };
    bootstrap(&mut ctx, &f, policy).await;

    assert_err(agent_spend(&mut ctx, &f, 10).await, E::DestinationNotAllowed);
}

#[tokio::test]
async fn empty_allowlist_permits_any_destination() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, default_policy()).await;
    agent_spend(&mut ctx, &f, 10).await.unwrap();
}

#[tokio::test]
async fn zero_amount_rejected() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, default_policy()).await;
    assert_err(agent_spend(&mut ctx, &f, 0).await, E::ZeroAmount);
}
