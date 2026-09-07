//! ## Authorization
//!
//! Who may do what. These rejections come from Anchor's own `seeds` and
//! `has_one` constraints rather than our policy code, which is the point -
//! the constraint system is doing the work so we cannot forget a check.

mod common;

use agent_wallet::{accounts as acc, instruction as ixn};
use common::*;
use solana_sdk::{signature::Keypair, signature::Signer};

#[tokio::test]
async fn stranger_cannot_revoke() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, default_policy()).await;

    let stranger = Keypair::new();
    let revoke = ix(
        acc::ModifyDelegation {
            owner: stranger.pubkey(),
            vault: f.vault,
            delegation: f.delegation,
        },
        ixn::RevokeDelegation {},
    );
    assert_constraint_err(send(&mut ctx, &[revoke], &[&stranger]).await);

    // Still usable by its agent.
    agent_spend(&mut ctx, &f, 10).await.unwrap();
}

#[tokio::test]
async fn stranger_cannot_pause() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, default_policy()).await;

    let stranger = Keypair::new();
    let pause = ix(
        acc::OwnerOnly {
            owner: stranger.pubkey(),
            vault: f.vault,
        },
        ixn::SetPaused { paused: true },
    );
    assert_constraint_err(send(&mut ctx, &[pause], &[&stranger]).await);
}

#[tokio::test]
async fn another_agent_cannot_spend_this_delegation() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, default_policy()).await;

    // A different session key presenting someone else's delegation account.
    let intruder = Keypair::new();
    let mut i = transfer_ix(&f, 10);
    i.accounts[0].pubkey = intruder.pubkey();

    assert_constraint_err(send(&mut ctx, &[i], &[&intruder]).await);
    assert_eq!(balance(&mut ctx, f.vault_token).await, START_BALANCE);
}

#[tokio::test]
async fn owner_can_withdraw_outside_any_policy() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, default_policy()).await;

    // 900 is far over the agent's 100 per-tx cap - the owner is not bound by it.
    let w = ix(
        acc::Withdraw {
            owner: f.owner.pubkey(),
            vault: f.vault,
            mint: f.mint,
            vault_token_account: f.vault_token,
            destination_token_account: f.dest_token,
            token_program: spl_token::id(),
        },
        ixn::Withdraw { amount: 900 },
    );
    send(&mut ctx, &[w], &[&f.owner]).await.unwrap();
    assert_eq!(balance(&mut ctx, f.vault_token).await, 100);
}

#[tokio::test]
async fn agent_cannot_withdraw() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;
    bootstrap(&mut ctx, &f, default_policy()).await;

    let w = ix(
        acc::Withdraw {
            owner: f.agent.pubkey(),
            vault: f.vault,
            mint: f.mint,
            vault_token_account: f.vault_token,
            destination_token_account: f.dest_token,
            token_program: spl_token::id(),
        },
        ixn::Withdraw { amount: 900 },
    );
    assert_constraint_err(send(&mut ctx, &[w], &[&f.agent]).await);
    assert_eq!(balance(&mut ctx, f.vault_token).await, START_BALANCE);
}
