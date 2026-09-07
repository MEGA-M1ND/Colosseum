//! ## State
//!
//! Two accounts. The vault custodies funds; the delegation carries one agent's
//! policy and its spend counters.
//!
//! Counters live on the delegation rather than the vault so two agents get
//! independent budgets: neither can starve the other, and revoking one leaves
//! the other untouched.

use anchor_lang::prelude::*;

use crate::errors::AgentWalletError;

pub const MAX_ALLOWLIST: usize = 8;

// ## Vault

#[account]
#[derive(InitSpace)]
pub struct Vault {
    /// The only key that may create, update or revoke delegations.
    pub owner: Pubkey,
    /// Vault-wide stop. Blocks every agent at once.
    pub paused: bool,
    pub bump: u8,
}

impl Vault {
    pub const SEED: &'static [u8] = b"vault";
}

// ## Delegation

#[account]
#[derive(InitSpace)]
pub struct Delegation {
    pub vault: Pubkey,
    /// The agent's session key. Never the owner's key.
    pub agent: Pubkey,
    /// One mint per delegation. See the spike: this keeps the metering sound.
    pub mint: Pubkey,

    pub per_tx_limit: u64,
    pub total_limit: u64,
    pub total_spent: u64,

    /// Seconds. Zero disables the rate limit entirely.
    pub window_duration: i64,
    pub window_limit: u64,
    pub window_spent: u64,
    pub window_started_at: i64,

    pub expires_at: i64,
    pub revoked: bool,

    /// Token account OWNERS the agent may send to. Empty means any.
    #[max_len(8)]
    pub allowed_destinations: Vec<Pubkey>,
    /// Programs `agent_invoke` may call. Empty means none - invoke is opt-in.
    #[max_len(8)]
    pub allowed_programs: Vec<Pubkey>,

    pub bump: u8,
}

impl Delegation {
    pub const SEED: &'static [u8] = b"delegation";

    /// Is this delegation usable at all? Separated from the spend accounting so
    /// `agent_invoke` can refuse a revoked delegation BEFORE running the CPI
    /// rather than unwinding it afterwards.
    pub fn check_active(&self, now: i64) -> Result<()> {
        require!(!self.revoked, AgentWalletError::DelegationRevoked);
        require!(now < self.expires_at, AgentWalletError::DelegationExpired);
        Ok(())
    }

    /// ## Policy evaluation
    ///
    /// Ordered cheapest and most decisive first. Every counter uses checked
    /// arithmetic. Mutates the counters, so the caller must only reach here
    /// once the spend is real.
    pub fn check_and_record(&mut self, amount: u64, now: i64) -> Result<()> {
        self.check_active(now)?;
        require!(amount > 0, AgentWalletError::ZeroAmount);
        require!(
            amount <= self.per_tx_limit,
            AgentWalletError::ExceedsPerTxLimit
        );

        let new_total = self
            .total_spent
            .checked_add(amount)
            .ok_or(AgentWalletError::MathOverflow)?;
        require!(
            new_total <= self.total_limit,
            AgentWalletError::ExceedsTotalLimit
        );

        // Tumbling window: resets wholesale on expiry rather than tracking a
        // true rolling sum, which would need a ring buffer of timestamped
        // spends for a marginal tightening. Set the window to half your real
        // tolerance to cover the boundary case.
        if self.window_duration > 0 {
            let window_end = self
                .window_started_at
                .checked_add(self.window_duration)
                .ok_or(AgentWalletError::MathOverflow)?;
            if now >= window_end {
                self.window_started_at = now;
                self.window_spent = 0;
            }
            let new_window = self
                .window_spent
                .checked_add(amount)
                .ok_or(AgentWalletError::MathOverflow)?;
            require!(
                new_window <= self.window_limit,
                AgentWalletError::ExceedsWindowLimit
            );
            self.window_spent = new_window;
        }

        self.total_spent = new_total;
        Ok(())
    }

    pub fn check_destination(&self, destination_owner: &Pubkey) -> Result<()> {
        if self.allowed_destinations.is_empty() {
            return Ok(());
        }
        require!(
            self.allowed_destinations.contains(destination_owner),
            AgentWalletError::DestinationNotAllowed
        );
        Ok(())
    }

    pub fn check_program(&self, program: &Pubkey) -> Result<()> {
        require!(
            self.allowed_programs.contains(program),
            AgentWalletError::ProgramNotAllowed
        );
        Ok(())
    }
}

// ## Policy payload
//
// Passed to create_delegation / update_delegation as one struct so the argument
// list stays readable and the client has a single shape to build.

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PolicyArgs {
    pub per_tx_limit: u64,
    pub total_limit: u64,
    pub window_duration: i64,
    pub window_limit: u64,
    pub expires_at: i64,
    pub allowed_destinations: Vec<Pubkey>,
    pub allowed_programs: Vec<Pubkey>,
}

impl PolicyArgs {
    pub fn validate(&self, now: i64) -> Result<()> {
        require!(
            self.allowed_destinations.len() <= MAX_ALLOWLIST
                && self.allowed_programs.len() <= MAX_ALLOWLIST,
            AgentWalletError::TooManyEntries
        );
        require!(self.expires_at > now, AgentWalletError::InvalidExpiry);
        // A window duration with no limit would silently allow everything.
        require!(
            (self.window_duration == 0) == (self.window_limit == 0),
            AgentWalletError::InvalidWindow
        );
        Ok(())
    }
}
