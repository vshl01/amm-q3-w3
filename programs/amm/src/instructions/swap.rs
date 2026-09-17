use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{
    error::AmmError,
    math::{fee_amount, swap_output},
    Pool, POOL_SEED,
};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        seeds = [
            POOL_SEED,
            pool.mint_a.as_ref(),
            pool.mint_b.as_ref()
        ],
        bump = pool.bump,
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        mut,
        token::mint = pool.mint_a,
        token::authority = authority,
    )]
    pub user_token_a: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = pool.mint_b,
        token::authority = authority,
    )]
    pub user_token_b: Box<Account<'info, TokenAccount>>,

    #[account(mut, address = pool.vault_a)]
    pub vault_a: Box<Account<'info, TokenAccount>>,

    #[account(mut, address = pool.vault_b)]
    pub vault_b: Box<Account<'info, TokenAccount>>,

    #[account(mut, address = pool.treasury_a)]
    pub treasury_a: Box<Account<'info, TokenAccount>>,

    #[account(mut, address = pool.treasury_b)]
    pub treasury_b: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Swap<'info> {
    /// `a_to_b = true` sells token A for token B, `false` does the opposite.
    ///
    /// The fee comes off the input side and goes straight to the treasury, so
    /// only the post-fee amount ever touches the curve.
    pub fn handle_swap(&mut self, amount_in: u64, min_amount_out: u64, a_to_b: bool) -> Result<()> {
        require!(amount_in > 0, AmmError::ZeroAmount);

        let fee = fee_amount(amount_in, self.pool.fee_bps)?;
        let amount_in_net = amount_in.checked_sub(fee).ok_or(AmmError::Overflow)?;

        require!(amount_in_net > 0, AmmError::ZeroAmount);

        let (reserve_in, reserve_out) = if a_to_b {
            (self.vault_a.amount, self.vault_b.amount)
        } else {
            (self.vault_b.amount, self.vault_a.amount)
        };

        let amount_out = swap_output(amount_in_net, reserve_in, reserve_out)?;

        require!(amount_out >= min_amount_out, AmmError::SlippageExceeded);

        let (user_in, user_out, vault_in, vault_out, treasury_in) = if a_to_b {
            (
                self.user_token_a.to_account_info(),
                self.user_token_b.to_account_info(),
                self.vault_a.to_account_info(),
                self.vault_b.to_account_info(),
                self.treasury_a.to_account_info(),
            )
        } else {
            (
                self.user_token_b.to_account_info(),
                self.user_token_a.to_account_info(),
                self.vault_b.to_account_info(),
                self.vault_a.to_account_info(),
                self.treasury_b.to_account_info(),
            )
        };

        // Fee: user -> treasury
        if fee > 0 {
            token::transfer(
                CpiContext::new(
                    self.token_program.key(),
                    Transfer {
                        from: user_in.clone(),
                        to: treasury_in,
                        authority: self.authority.to_account_info(),
                    },
                ),
                fee,
            )?;
        }

        // Trade in: user -> vault
        token::transfer(
            CpiContext::new(
                self.token_program.key(),
                Transfer {
                    from: user_in,
                    to: vault_in,
                    authority: self.authority.to_account_info(),
                },
            ),
            amount_in_net,
        )?;

        // Trade out: vault -> user, signed by the pool PDA
        let mint_a = self.pool.mint_a;
        let mint_b = self.pool.mint_b;
        let bump = [self.pool.bump];
        let seeds: [&[u8]; 4] = [POOL_SEED, mint_a.as_ref(), mint_b.as_ref(), &bump];
        let signer: [&[&[u8]]; 1] = [&seeds];

        token::transfer(
            CpiContext::new_with_signer(
                self.token_program.key(),
                Transfer {
                    from: vault_out,
                    to: user_out,
                    authority: self.pool.to_account_info(),
                },
                &signer,
            ),
            amount_out,
        )?;

        Ok(())
    }
}
