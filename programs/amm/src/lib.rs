pub mod constants;
pub mod error;
pub mod instructions;
pub mod math;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("2KRXzBtBcv9dgKDx2KGF84N8XCHdJMLMse97hzYFpNMF");

#[program]
pub mod amm_q3_w3 {
    use super::*;

    /// Create the pool PDA, both vaults, the LP mint and both fee treasuries.
    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        ctx.accounts
            .handle_initialize(fee_bps, ctx.bumps.pool, ctx.bumps.lp_mint)
    }

    /// Add liquidity and receive LP tokens.
    pub fn deposit(
        ctx: Context<Deposit>,
        amount_a: u64,
        amount_b: u64,
        min_lp: u64,
    ) -> Result<()> {
        ctx.accounts.handle_deposit(amount_a, amount_b, min_lp)
    }

    /// Burn LP tokens and take back your share of both vaults.
    pub fn withdraw(ctx: Context<Withdraw>, lp_amount: u64, min_a: u64, min_b: u64) -> Result<()> {
        ctx.accounts.handle_withdraw(lp_amount, min_a, min_b)
    }

    /// Trade one side for the other along the constant-product curve.
    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        min_amount_out: u64,
        a_to_b: bool,
    ) -> Result<()> {
        ctx.accounts.handle_swap(amount_in, min_amount_out, a_to_b)
    }
}
