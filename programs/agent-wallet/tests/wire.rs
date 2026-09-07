//! ## Wire reference vectors
//!
//! The frontend builds instructions by hand - `anchor build` needs the SBF
//! toolchain, so there is no generated IDL for it to import. That puts the
//! whole client one typo away from opaque runtime failures.
//!
//! This test emits the authoritative bytes for every instruction, using the
//! Anchor-generated types, from fixed inputs. `app/scripts/check-wire.ts`
//! rebuilds the same calls in TypeScript and byte-compares. Either side
//! drifting fails that check instead of failing on-chain.

mod common;

use agent_wallet::state::{Delegation, PolicyArgs, Vault};
use agent_wallet::{accounts as acc, instruction as ixn};
use anchor_lang::{InstructionData, ToAccountMetas};
use common::SYSTEM_PROGRAM;
use serde_json::{json, Value};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

fn key(b: u8) -> Pubkey {
    Pubkey::new_from_array([b; 32])
}

fn describe(name: &str, ix: Instruction) -> Value {
    json!({
        "name": name,
        "data": hex(&ix.data),
        "accounts": ix.accounts.iter().map(|a| json!({
            "pubkey": a.pubkey.to_string(),
            "isSigner": a.is_signer,
            "isWritable": a.is_writable,
        })).collect::<Vec<_>>(),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction {
        program_id: agent_wallet::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Anchor assigns codes in declaration order from ERROR_CODE_OFFSET. The client
/// hardcodes the mapping to decode rejections, so drift here would silently
/// mislabel every failure - reordering the enum is enough to break it.
fn error_codes() -> Value {
    use agent_wallet::errors::AgentWalletError as E;
    let all = [
        ("VaultPaused", E::VaultPaused),
        ("DelegationRevoked", E::DelegationRevoked),
        ("DelegationExpired", E::DelegationExpired),
        ("ZeroAmount", E::ZeroAmount),
        ("ExceedsPerTxLimit", E::ExceedsPerTxLimit),
        ("ExceedsTotalLimit", E::ExceedsTotalLimit),
        ("ExceedsWindowLimit", E::ExceedsWindowLimit),
        ("DestinationNotAllowed", E::DestinationNotAllowed),
        ("ProgramNotAllowed", E::ProgramNotAllowed),
        ("SelfInvokeForbidden", E::SelfInvokeForbidden),
        ("TargetNotExecutable", E::TargetNotExecutable),
        ("MathOverflow", E::MathOverflow),
        ("AuthorityAltered", E::AuthorityAltered),
        ("UnmeteredHoldingFell", E::UnmeteredHoldingFell),
        ("VaultLamportsFell", E::VaultLamportsFell),
        ("NotVaultTokenAccount", E::NotVaultTokenAccount),
        ("MintMismatch", E::MintMismatch),
        ("TooManyEntries", E::TooManyEntries),
        ("InvalidWindow", E::InvalidWindow),
        ("InvalidExpiry", E::InvalidExpiry),
    ];
    let mut out = serde_json::Map::new();
    for (name, variant) in all {
        out.insert(
            name.to_string(),
            json!(variant as u32 + anchor_lang::error::ERROR_CODE_OFFSET),
        );
    }
    Value::Object(out)
}

/// The client decodes these straight off the wire, so its layout assumptions
/// need the same protection as the instruction encoders.
fn serialize_delegation(vault: Pubkey, agent: Pubkey, mint: Pubkey) -> Vec<u8> {
    use anchor_lang::AccountSerialize;
    let d = Delegation {
        vault,
        agent,
        mint,
        per_tx_limit: 100,
        total_limit: 500,
        total_spent: 275,
        window_duration: 3600,
        window_limit: 250,
        window_spent: 75,
        window_started_at: 1_700_000_000,
        expires_at: 1_800_000_000,
        revoked: false,
        allowed_destinations: vec![key(8)],
        allowed_programs: vec![spl_token::id()],
        bump: 254,
    };
    let mut buf = Vec::new();
    d.try_serialize(&mut buf).unwrap();
    buf
}

fn serialize_vault(owner: Pubkey) -> Vec<u8> {
    use anchor_lang::AccountSerialize;
    let v = Vault { owner, paused: true, bump: 253 };
    let mut buf = Vec::new();
    v.try_serialize(&mut buf).unwrap();
    buf
}

#[test]
fn emit_wire_reference() {
    let owner = key(1);
    let agent = key(2);
    let mint = key(3);
    let owner_token = key(4);
    let vault_token = key(5);
    let dest_token = key(6);
    let target = key(7);

    let (vault, _) =
        Pubkey::find_program_address(&[Vault::SEED, owner.as_ref()], &agent_wallet::ID);
    let (delegation, _) = Pubkey::find_program_address(
        &[Delegation::SEED, vault.as_ref(), agent.as_ref()],
        &agent_wallet::ID,
    );

    // Non-empty allowlists on purpose: exercises the Vec<Pubkey> encoding,
    // which is the easiest part of the wire format to get wrong.
    let policy = PolicyArgs {
        per_tx_limit: 100,
        total_limit: 500,
        window_duration: 3600,
        window_limit: 250,
        expires_at: 1_800_000_000,
        allowed_destinations: vec![key(8), key(9)],
        allowed_programs: vec![spl_token::id()],
    };

    let mut invoke_ix = ix(
        acc::AgentInvoke {
            agent,
            vault,
            delegation,
            mint,
            vault_token_account: vault_token,
            target_program: target,
        },
        ixn::AgentInvoke {
            data: vec![3, 50, 0, 0, 0, 0, 0, 0, 0], // spl-token Transfer of 50
        },
    );
    // remaining_accounts, appended by the client
    invoke_ix.accounts.push(solana_sdk::instruction::AccountMeta::new(vault_token, false));
    invoke_ix.accounts.push(solana_sdk::instruction::AccountMeta::new(dest_token, false));
    invoke_ix
        .accounts
        .push(solana_sdk::instruction::AccountMeta::new_readonly(vault, false));

    let vectors = json!({
        "programId": agent_wallet::ID.to_string(),
        "inputs": {
            "owner": owner.to_string(),
            "agent": agent.to_string(),
            "mint": mint.to_string(),
            "ownerToken": owner_token.to_string(),
            "vaultToken": vault_token.to_string(),
            "destToken": dest_token.to_string(),
            "target": target.to_string(),
            "tokenProgram": spl_token::id().to_string(),
        },
        "pdas": { "vault": vault.to_string(), "delegation": delegation.to_string() },
        "policy": {
            "perTxLimit": policy.per_tx_limit,
            "totalLimit": policy.total_limit,
            "windowDuration": policy.window_duration,
            "windowLimit": policy.window_limit,
            "expiresAt": policy.expires_at,
            "allowedDestinations": policy.allowed_destinations.iter().map(|k| k.to_string()).collect::<Vec<_>>(),
            "allowedPrograms": policy.allowed_programs.iter().map(|k| k.to_string()).collect::<Vec<_>>(),
        },
        "errors": error_codes(),
        "accounts": {
            "delegation": {
                "hex": hex(&serialize_delegation(vault, agent, mint)),
                "fields": {
                    "vault": vault.to_string(),
                    "agent": agent.to_string(),
                    "mint": mint.to_string(),
                    "perTxLimit": 100,
                    "totalLimit": 500,
                    "totalSpent": 275,
                    "windowDuration": 3600,
                    "windowLimit": 250,
                    "windowSpent": 75,
                    "windowStartedAt": 1_700_000_000,
                    "expiresAt": 1_800_000_000,
                    "revoked": false,
                    "allowedDestinations": [key(8).to_string()],
                    "allowedPrograms": [spl_token::id().to_string()],
                    "bump": 254,
                }
            },
            "vault": {
                "hex": hex(&serialize_vault(owner)),
                "fields": { "owner": owner.to_string(), "paused": true, "bump": 253 }
            }
        },
        "instructions": [
            describe("initializeVault", ix(
                acc::InitializeVault { owner, vault, system_program: SYSTEM_PROGRAM },
                ixn::InitializeVault {},
            )),
            describe("setPaused", ix(
                acc::OwnerOnly { owner, vault },
                ixn::SetPaused { paused: true },
            )),
            describe("createDelegation", ix(
                acc::CreateDelegation { owner, vault, agent, mint, delegation, system_program: SYSTEM_PROGRAM },
                ixn::CreateDelegation { policy: policy.clone() },
            )),
            describe("updateDelegation", ix(
                acc::ModifyDelegation { owner, vault, delegation },
                ixn::UpdateDelegation { policy: policy.clone() },
            )),
            describe("revokeDelegation", ix(
                acc::ModifyDelegation { owner, vault, delegation },
                ixn::RevokeDelegation {},
            )),
            describe("deposit", ix(
                acc::Deposit {
                    owner, vault, mint,
                    owner_token_account: owner_token,
                    vault_token_account: vault_token,
                    token_program: spl_token::id(),
                },
                ixn::Deposit { amount: 250 },
            )),
            describe("withdraw", ix(
                acc::Withdraw {
                    owner, vault, mint,
                    vault_token_account: vault_token,
                    destination_token_account: dest_token,
                    token_program: spl_token::id(),
                },
                ixn::Withdraw { amount: 900 },
            )),
            describe("agentTransfer", ix(
                acc::AgentTransfer {
                    agent, vault, delegation, mint,
                    vault_token_account: vault_token,
                    destination_token_account: dest_token,
                    token_program: spl_token::id(),
                },
                ixn::AgentTransfer { amount: 50 },
            )),
            describe("agentInvoke", invoke_ix),
        ],
    });

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/wire-reference.json");
    std::fs::write(path, serde_json::to_string_pretty(&vectors).unwrap())
        .expect("write wire-reference.json");
    println!("wrote {path}");
}
