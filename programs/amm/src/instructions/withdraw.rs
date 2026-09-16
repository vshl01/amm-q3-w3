use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::{error::AmmError, math::amount_for_lp, Pool, POOL_SEED};

#[derive(Accounts)]
pub struct Withdraw<'info> {
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
    pub pool: Box<Account<'info, Pool>>,

    #[account(mut, address = pool.lp_mint)]
    pub lp_mint: Box<Account<'info, Mint>>,

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

    /// LP tokens being handed back in.
    #[account(
        mut,
        token::mint = lp_mint,
        token::authority = authority,
    )]
    pub user_lp: Box<Account<'info, TokenAccount>>,

    #[account(mut, address = pool.vault_a)]
    pub vault_a: Box<Account<'info, TokenAccount>>,

    #[account(mut, address = pool.vault_b)]
    pub vault_b: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Withdraw<'info> {
    pub fn handle_withdraw(&mut self, lp_amount: u64, min_a: u64, min_b: u64) -> Result<()> {
        require!(lp_amount > 0, AmmError::ZeroAmount);
        require!(
            self.user_lp.amount >= lp_amount,
            AmmError::InsufficientLpBalance
        );

        let lp_supply = self.lp_mint.supply;

        let amount_a = amount_for_lp(lp_amount, self.vault_a.amount, lp_supply)?;
        let amount_b = amount_for_lp(lp_amount, self.vault_b.amount, lp_supply)?;

        require!(amount_a > 0 && amount_b > 0, AmmError::ZeroAmount);
        require!(
            amount_a >= min_a && amount_b >= min_b,
            AmmError::SlippageExceeded
        );

        // Burn first, so the shares can never be spent twice.
        token::burn(
            CpiContext::new(
                self.token_program.key(),
                Burn {
                    mint: self.lp_mint.to_account_info(),
                    from: self.user_lp.to_account_info(),
                    authority: self.authority.to_account_info(),
                },
            ),
            lp_amount,
        )?;

        let mint_a = self.pool.mint_a;
        let mint_b = self.pool.mint_b;
        let bump = [self.pool.bump];
        let seeds: [&[u8]; 4] = [POOL_SEED, mint_a.as_ref(), mint_b.as_ref(), &bump];
        let signer: [&[&[u8]]; 1] = [&seeds];

        // vault A -> user A
        token::transfer(
            CpiContext::new_with_signer(
                self.token_program.key(),
                Transfer {
                    from: self.vault_a.to_account_info(),
                    to: self.user_token_a.to_account_info(),
                    authority: self.pool.to_account_info(),
                },
                &signer,
            ),
            amount_a,
        )?;

        // vault B -> user B
        token::transfer(
            CpiContext::new_with_signer(
                self.token_program.key(),
                Transfer {
                    from: self.vault_b.to_account_info(),
                    to: self.user_token_b.to_account_info(),
                    authority: self.pool.to_account_info(),
                },
                &signer,
            ),
            amount_b,
        )?;

        Ok(())
    }
}
