use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{Pool, POOL_SEED};

#[derive(Accounts)]
pub struct Swap<'info> {
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

    // User gives Token A
    #[account(
        mut,
        token::mint = pool.mint_a,
        token::authority = authority,
    )]
    pub user_token_a: Account<'info, TokenAccount>,

    // User receives Token B
    #[account(
        mut,
        token::mint = pool.mint_b,
        token::authority = authority,
    )]
    pub user_token_b: Account<'info, TokenAccount>,

    // Pool's Token A
    #[account(
        mut,
        address = pool.vault_a,
    )]
    pub vault_a: Account<'info, TokenAccount>,

    // Pool's Token B
    #[account(
        mut,
        address = pool.vault_b,
    )]
    pub vault_b: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Swap<'info> {
    pub fn swap(&mut self, amount_in: u64) -> Result<()> {
        let reserve_a = self.vault_a.amount;
        let reserve_b = self.vault_b.amount;

        // x * y = k
        let k = (reserve_a as u128).checked_mul(reserve_b as u128).unwrap();

        // New Token A reserve
        let new_reserve_a = reserve_a as u128 + amount_in as u128;

        // New Token B reserve
        let new_reserve_b = k.checked_div(new_reserve_a).unwrap();

        // Amount of Token B user receives
        let amount_out = reserve_b as u128 - new_reserve_b;

        // User A → Vault A
        let transfer_in = Transfer {
            from: self.user_token_a.to_account_info(),
            to: self.vault_a.to_account_info(),
            authority: self.authority.to_account_info(),
        };

        let ctx_in = CpiContext::new(self.token_program.key(), transfer_in);

        token::transfer(ctx_in, amount_in)?;

        // Vault B → User B

        let signer_seeds: &[&[u8]] = &[
            POOL_SEED,
            self.pool.mint_a.as_ref(),
            self.pool.mint_b.as_ref(),
            &[self.pool.bump],
        ];

        let signer_seeds_array = [signer_seeds];

        let transfer_out = Transfer {
            from: self.vault_b.to_account_info(),
            to: self.user_token_b.to_account_info(),
            authority: self.pool.to_account_info(),
        };

        let ctx_out = CpiContext::new(self.token_program.key(), transfer_out)
            .with_signer(&signer_seeds_array);

        token::transfer(ctx_out, amount_out as u64)?;

        Ok(())
    }
}
