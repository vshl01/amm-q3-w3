use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

use crate::{
    error::AmmError,
    math::{initial_lp_tokens, lp_tokens_for_deposit},
    Pool, MINIMUM_LIQUIDITY, POOL_SEED,
};

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

    #[account(mut, address = pool.lp_mint)]
    pub lp_mint: Account<'info, Mint>,

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

    /// Where the depositor's new LP tokens land.
    #[account(
        mut,
        token::mint = lp_mint,
        token::authority = authority,
    )]
    pub user_lp: Account<'info, TokenAccount>,

    #[account(mut, address = pool.vault_a)]
    pub vault_a: Account<'info, TokenAccount>,

    #[account(mut, address = pool.vault_b)]
    pub vault_b: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Deposit<'info> {
    pub fn handle_deposit(&mut self, amount_a: u64, amount_b: u64, min_lp: u64) -> Result<()> {
        require!(amount_a > 0 && amount_b > 0, AmmError::ZeroAmount);

        // Reserves as they were before this instruction moved anything.
        let reserve_a = self.vault_a.amount;
        let reserve_b = self.vault_b.amount;
        let lp_supply = self.lp_mint.supply;

        let lp_to_mint = if lp_supply == 0 {
            let lp = initial_lp_tokens(amount_a, amount_b)?;

            require!(
                lp > MINIMUM_LIQUIDITY,
                AmmError::InsufficientInitialLiquidity
            );

            lp
        } else {
            lp_tokens_for_deposit(amount_a, amount_b, reserve_a, reserve_b, lp_supply)?
        };

        require!(lp_to_mint > 0, AmmError::ZeroAmount);
        require!(lp_to_mint >= min_lp, AmmError::SlippageExceeded);

        // user A -> vault A
        token::transfer(
            CpiContext::new(
                self.token_program.key(),
                Transfer {
                    from: self.user_token_a.to_account_info(),
                    to: self.vault_a.to_account_info(),
                    authority: self.authority.to_account_info(),
                },
            ),
            amount_a,
        )?;

        // user B -> vault B
        token::transfer(
            CpiContext::new(
                self.token_program.key(),
                Transfer {
                    from: self.user_token_b.to_account_info(),
                    to: self.vault_b.to_account_info(),
                    authority: self.authority.to_account_info(),
                },
            ),
            amount_b,
        )?;

        // Mint LP shares. The pool PDA is the mint authority, so it signs.
        let mint_a = self.pool.mint_a;
        let mint_b = self.pool.mint_b;
        let bump = [self.pool.bump];
        let seeds: [&[u8]; 4] = [POOL_SEED, mint_a.as_ref(), mint_b.as_ref(), &bump];
        let signer: [&[&[u8]]; 1] = [&seeds];

        token::mint_to(
            CpiContext::new_with_signer(
                self.token_program.key(),
                MintTo {
                    mint: self.lp_mint.to_account_info(),
                    to: self.user_lp.to_account_info(),
                    authority: self.pool.to_account_info(),
                },
                &signer,
            ),
            lp_to_mint,
        )?;

        Ok(())
    }
}
