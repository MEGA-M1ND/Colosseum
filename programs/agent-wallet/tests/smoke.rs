mod common;

use agent_wallet::state::PolicyArgs;
use agent_wallet::{accounts as acc, instruction as ixn};
use anchor_lang::prelude::*;
use common::*;
use solana_sdk::signature::Signer;

#[tokio::test]
async fn happy_path() {
    let (pt, f) = setup();
    let mut ctx = pt.start_with_context().await;

    send(
        &mut ctx,
        &[ix(
            acc::InitializeVault {
                owner: f.owner.pubkey(),
                vault: f.vault,
                system_program: SYSTEM_PROGRAM,
            },
            ixn::InitializeVault {},
        )],
        &[&f.owner],
    )
    .await
    .unwrap();

    let policy = PolicyArgs {
        per_tx_limit: 100,
        total_limit: 500,
        window_duration: 0,
        window_limit: 0,
        expires_at: i64::MAX,
        allowed_destinations: vec![],
        allowed_programs: vec![],
    };

    send(
        &mut ctx,
        &[ix(
            acc::CreateDelegation {
                owner: f.owner.pubkey(),
                vault: f.vault,
                agent: f.agent.pubkey(),
                mint: f.mint,
                delegation: f.delegation,
                system_program: SYSTEM_PROGRAM,
            },
            ixn::CreateDelegation { policy },
        )],
        &[&f.owner],
    )
    .await
    .unwrap();

    send(
        &mut ctx,
        &[ix(
            acc::AgentTransfer {
                agent: f.agent.pubkey(),
                vault: f.vault,
                delegation: f.delegation,
                mint: f.mint,
                vault_token_account: f.vault_token,
                destination_token_account: f.dest_token,
                token_program: spl_token::id(),
            },
            ixn::AgentTransfer { amount: 50 },
        )],
        &[&f.agent],
    )
    .await
    .unwrap();

    assert_eq!(balance(&mut ctx, f.vault_token).await, 950);
    assert_eq!(balance(&mut ctx, f.dest_token).await, 50);
    assert_eq!(delegation_state(&mut ctx, f.delegation).await.total_spent, 50);
}
