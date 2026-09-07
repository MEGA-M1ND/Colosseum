//! # Agent Wallet
//!
//! Give an AI agent a budget, not your keys.
//!
//! The owner funds a vault PDA and issues a delegation to an agent's session
//! key carrying a policy: per-transaction cap, lifetime cap, a rolling-window
//! rate limit, an expiry, a destination allowlist, and instant revocation. The
//! agent signs with the session key; the program enforces the policy and signs
//! the token movement as the vault PDA. The owner's keys never touch the agent.
//!
//! ## What makes this different from a session key
//!
//! Existing approaches scope by *capability*: this key may call these
//! instructions on these programs. That answers "what can it do" and leaves
//! "how much can it cost me" open - a whitelisted swap in a loop drains the
//! wallet without ever leaving the allowlist.
//!
//! This scopes by *economic damage*. `agent_invoke` goes further and bounds an
//! arbitrary, unparsed instruction by measuring what left the vault. See
//! `guard.rs`.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

pub mod errors;
pub mod guard;
pub mod state;

use errors::AgentWalletError;
use state::{Delegation, PolicyArgs, Vault};

declare_id!("Ag3ntWa11et11111111111111111111111111111111");

#[program]
pub mod agent_wallet {
    use super::*;

    // ## Owner instructions

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.owner = ctx.accounts.owner.key();
        vault.paused = false;
        vault.bump = ctx.bumps.vault;
        Ok(())
    }

    /// Vault-wide stop. Blocks every agent at once, without touching any
    /// individual delegation.
    pub fn set_paused(ctx: Context<OwnerOnly>, paused: bool) -> Result<()> {
        ctx.accounts.vault.paused = paused;
        Ok(())
    }

    pub fn create_delegation(ctx: Context<CreateDelegation>, policy: PolicyArgs) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        policy.validate(now)?;

        let d = &mut ctx.accounts.delegation;
        d.vault = ctx.accounts.vault.key();
        d.agent = ctx.accounts.agent.key();
        d.mint = ctx.accounts.mint.key();
        d.per_tx_limit = policy.per_tx_limit;
        d.total_limit = policy.total_limit;
        d.total_spent = 0;
        d.window_duration = policy.window_duration;
        d.window_limit = policy.window_limit;
        d.window_spent = 0;
        d.window_started_at = now;
        d.expires_at = policy.expires_at;
        d.revoked = false;
        d.allowed_destinations = policy.allowed_destinations;
        d.allowed_programs = policy.allowed_programs;
        d.bump = ctx.bumps.delegation;
        Ok(())
    }

    /// Adjust limits on a live delegation. Spend counters are deliberately NOT
    /// reset - tightening a limit must never hand back budget already used.
    pub fn update_delegation(ctx: Context<ModifyDelegation>, policy: PolicyArgs) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        policy.validate(now)?;

        let d = &mut ctx.accounts.delegation;
        d.per_tx_limit = policy.per_tx_limit;
        d.total_limit = policy.total_limit;
        d.window_duration = policy.window_duration;
        d.window_limit = policy.window_limit;
        d.expires_at = policy.expires_at;
        d.allowed_destinations = policy.allowed_destinations;
        d.allowed_programs = policy.allowed_programs;
        Ok(())
    }

    /// Effective immediately: every spend re-reads this flag, and nothing is
    /// cached client-side.
    ///
    /// The account is kept rather than closed. The rent refund is not worth
    /// losing the record of what the agent did - an auditable spend history is
    /// itself part of the product.
    pub fn revoke_delegation(ctx: Context<ModifyDelegation>) -> Result<()> {
        ctx.accounts.delegation.revoked = true;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, AgentWalletError::ZeroAmount);
        token::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.owner_token_account.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.vault_token_account.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            amount,
            ctx.accounts.mint.decimals,
        )
    }

    /// The owner's escape hatch. Not subject to any delegation policy.
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, AgentWalletError::ZeroAmount);
        let owner = ctx.accounts.vault.owner;
        let seeds: &[&[u8]] = &[Vault::SEED, owner.as_ref(), &[ctx.accounts.vault.bump]];

        token::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.destination_token_account.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                &[seeds],
            ),
            amount,
            ctx.accounts.mint.decimals,
        )
    }

    // ## Agent instructions

    /// ## The product
    ///
    /// A direct token transfer from the vault, bounded by the delegation.
    /// Everything else in this program is scaffolding around this.
    pub fn agent_transfer(ctx: Context<AgentTransfer>, amount: u64) -> Result<()> {
        require!(!ctx.accounts.vault.paused, AgentWalletError::VaultPaused);
        let now = Clock::get()?.unix_timestamp;

        ctx.accounts
            .delegation
            .check_destination(&ctx.accounts.destination_token_account.owner)?;
        ctx.accounts.delegation.check_and_record(amount, now)?;

        let owner = ctx.accounts.vault.owner;
        let seeds: &[&[u8]] = &[Vault::SEED, owner.as_ref(), &[ctx.accounts.vault.bump]];

        token::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.destination_token_account.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                &[seeds],
            ),
            amount,
            ctx.accounts.mint.decimals,
        )
    }

    /// ## Phase 2 - arbitrary CPI, bounded by measurement
    ///
    /// Executes an instruction the program never parses, as the vault, and
    /// bounds it by what actually left. See `guard.rs` for why each check
    /// exists; every one of them started as a working exploit in the spike.
    ///
    /// Note the destination allowlist does NOT apply here - the guard cannot
    /// know a destination without parsing. The program allowlist is the control
    /// for this path, and it is empty by default, so invoke is opt-in.
    pub fn agent_invoke<'info>(
        ctx: Context<'_, '_, 'info, 'info, AgentInvoke<'info>>,
        data: Vec<u8>,
    ) -> Result<()> {
        require!(!ctx.accounts.vault.paused, AgentWalletError::VaultPaused);
        let now = Clock::get()?.unix_timestamp;

        // Checked before the CPI: a revoked delegation must not execute at all.
        ctx.accounts.delegation.check_active(now)?;

        let target = ctx.accounts.target_program.key();
        require_keys_neq!(target, crate::ID, AgentWalletError::SelfInvokeForbidden);
        require!(
            ctx.accounts.target_program.executable,
            AgentWalletError::TargetNotExecutable
        );
        ctx.accounts.delegation.check_program(&target)?;

        let vault_key = ctx.accounts.vault.key();
        let metered_info = ctx.accounts.vault_token_account.to_account_info();

        // ## Measure before
        let metered = guard::Snapshot::take(&metered_info)?;
        let siblings = guard::scope_siblings(ctx.remaining_accounts, &metered_info, &vault_key)?;

        // ## Invoke, unparsed
        let metas: Vec<AccountMeta> = ctx
            .remaining_accounts
            .iter()
            .map(|a| AccountMeta {
                pubkey: *a.key,
                // The vault PDA is the only thing this program signs for.
                is_signer: *a.key == vault_key,
                is_writable: a.is_writable,
            })
            .collect();

        let ix = Instruction {
            program_id: target,
            accounts: metas,
            data,
        };

        let mut infos = ctx.remaining_accounts.to_vec();
        infos.push(ctx.accounts.target_program.to_account_info());

        let owner = ctx.accounts.vault.owner;
        let seeds: &[&[u8]] = &[Vault::SEED, owner.as_ref(), &[ctx.accounts.vault.bump]];
        invoke_signed(&ix, &infos, &[seeds])?;

        // ## Measure after
        //
        // Order matters: the value has already moved, and returning Err is what
        // reverts it. Siblings first - a drained holding is the more dangerous
        // condition and the cheaper rejection.
        for sibling in siblings.iter() {
            sibling.enforce(false)?;
        }
        let spent = metered.enforce(true)?;

        // A zero-cost call (a read, an approval-free no-op) consumes no budget.
        if spent > 0 {
            ctx.accounts.delegation.check_and_record(spent, now)?;
        }
        Ok(())
    }
}

// ## Account contexts

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        init,
        payer = owner,
        space = 8 + Vault::INIT_SPACE,
        seeds = [Vault::SEED, owner.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct OwnerOnly<'info> {
    pub owner: Signer<'info>,
    #[account(
        mut,
        seeds = [Vault::SEED, owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Account<'info, Vault>,
}

#[derive(Accounts)]
pub struct CreateDelegation<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        seeds = [Vault::SEED, owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Account<'info, Vault>,
    /// CHECK: the agent's session key. Never signs here - the owner is granting
    /// authority to it, so it does not need to be present.
    pub agent: UncheckedAccount<'info>,
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = owner,
        space = 8 + Delegation::INIT_SPACE,
        seeds = [Delegation::SEED, vault.key().as_ref(), agent.key().as_ref()],
        bump
    )]
    pub delegation: Account<'info, Delegation>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ModifyDelegation<'info> {
    pub owner: Signer<'info>,
    #[account(
        seeds = [Vault::SEED, owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Account<'info, Vault>,
    #[account(
        mut,
        seeds = [Delegation::SEED, vault.key().as_ref(), delegation.agent.as_ref()],
        bump = delegation.bump,
        has_one = vault
    )]
    pub delegation: Account<'info, Delegation>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        seeds = [Vault::SEED, owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Account<'info, Vault>,
    pub mint: Account<'info, Mint>,
    #[account(mut, constraint = owner_token_account.mint == mint.key() @ AgentWalletError::MintMismatch)]
    pub owner_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault_token_account.owner == vault.key() @ AgentWalletError::NotVaultTokenAccount,
        constraint = vault_token_account.mint == mint.key() @ AgentWalletError::MintMismatch
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub owner: Signer<'info>,
    #[account(
        seeds = [Vault::SEED, owner.key().as_ref()],
        bump = vault.bump,
        has_one = owner
    )]
    pub vault: Account<'info, Vault>,
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = vault_token_account.owner == vault.key() @ AgentWalletError::NotVaultTokenAccount,
        constraint = vault_token_account.mint == mint.key() @ AgentWalletError::MintMismatch
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut, constraint = destination_token_account.mint == mint.key() @ AgentWalletError::MintMismatch)]
    pub destination_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct AgentTransfer<'info> {
    /// The session key. This is the only thing the agent holds.
    pub agent: Signer<'info>,
    #[account(
        seeds = [Vault::SEED, vault.owner.as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(
        mut,
        seeds = [Delegation::SEED, vault.key().as_ref(), agent.key().as_ref()],
        bump = delegation.bump,
        has_one = vault,
        has_one = agent,
        has_one = mint
    )]
    pub delegation: Account<'info, Delegation>,
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = vault_token_account.owner == vault.key() @ AgentWalletError::NotVaultTokenAccount,
        constraint = vault_token_account.mint == mint.key() @ AgentWalletError::MintMismatch
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut, constraint = destination_token_account.mint == mint.key() @ AgentWalletError::MintMismatch)]
    pub destination_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct AgentInvoke<'info> {
    pub agent: Signer<'info>,
    #[account(
        seeds = [Vault::SEED, vault.owner.as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(
        mut,
        seeds = [Delegation::SEED, vault.key().as_ref(), agent.key().as_ref()],
        bump = delegation.bump,
        has_one = vault,
        has_one = agent,
        has_one = mint
    )]
    pub delegation: Account<'info, Delegation>,
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = vault_token_account.owner == vault.key() @ AgentWalletError::NotVaultTokenAccount,
        constraint = vault_token_account.mint == mint.key() @ AgentWalletError::MintMismatch
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    /// CHECK: verified against the delegation's program allowlist, checked for
    /// executability, and refused if it is this program.
    pub target_program: UncheckedAccount<'info>,
    // Everything the inner instruction needs arrives in remaining_accounts.
}
