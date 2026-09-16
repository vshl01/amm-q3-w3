use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{Pool, POOL_SEED};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [
            POOL_SEED,
            pool.mint_a.as_ref(),
            pool.mint_b.as_ref()
        ],
        bump = pool.bump,
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        token::mint = pool.mint_a,
        token::authority = authority,
    )]
    pub user_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = pool.mint_b,
        token::authority = authority,
    )]
    pub user_token_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        address = pool.vault_a,
    )]
    pub vault_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        address = pool.vault_b,
    )]
    pub vault_b: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Deposit<'info> {
    pub fn handle_deposit(&mut self, amount_a: u64, amount_b: u64) -> Result<()> {
        // user A → vault A
        let transfer_a = Transfer {
            from: self.user_token_a.to_account_info(),
            to: self.vault_a.to_account_info(),
            authority: self.authority.to_account_info(),
        };

        let cpi_ctx_a = CpiContext::new(self.token_program.key(), transfer_a);

        token::transfer(cpi_ctx_a, amount_a)?;

        // user B → vault B
        let transfer_b = Transfer {
            from: self.user_token_b.to_account_info(),
            to: self.vault_b.to_account_info(),
            authority: self.authority.to_account_info(),
        };

        let cpi_ctx_b = CpiContext::new(self.token_program.key(), transfer_b);

        token::transfer(cpi_ctx_b, amount_b)?;

        Ok(())
    }
}
