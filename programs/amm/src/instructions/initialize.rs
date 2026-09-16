use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{error::AmmError, Pool, LP_DECIMALS, LP_MINT_SEED, MAX_FEE_BPS, POOL_SEED};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(constraint = mint_a.key() != mint_b.key() @ AmmError::SameMint)]
    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        space = 8 + Pool::INIT_SPACE,
        seeds = [
            POOL_SEED,
            mint_a.key().as_ref(),
            mint_b.key().as_ref()
        ],
        bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        init,
        payer = authority,
        token::mint = mint_a,
        token::authority = pool,
    )]
    pub vault_a: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        token::mint = mint_b,
        token::authority = pool,
    )]
    pub vault_b: Account<'info, TokenAccount>,

    /// LP mint for this pool. Pool PDA is the mint authority, so only this
    /// program can ever mint or burn shares.
    #[account(
        init,
        payer = authority,
        seeds = [LP_MINT_SEED, pool.key().as_ref()],
        bump,
        mint::decimals = LP_DECIMALS,
        mint::authority = pool,
    )]
    pub lp_mint: Account<'info, Mint>,

    /// Fee treasuries. Owned by `authority`, so collected fees can be moved
    /// with a normal SPL transfer - no extra instruction needed.
    #[account(
        init,
        payer = authority,
        token::mint = mint_a,
        token::authority = authority,
    )]
    pub treasury_a: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        token::mint = mint_b,
        token::authority = authority,
    )]
    pub treasury_b: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Initialize<'info> {
    pub fn handle_initialize(&mut self, fee_bps: u16, bump: u8, lp_bump: u8) -> Result<()> {
        require!(fee_bps <= MAX_FEE_BPS, AmmError::FeeTooHigh);

        self.pool.set_inner(Pool {
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            vault_a: self.vault_a.key(),
            vault_b: self.vault_b.key(),
            lp_mint: self.lp_mint.key(),
            treasury_a: self.treasury_a.key(),
            treasury_b: self.treasury_b.key(),
            authority: self.authority.key(),
            fee_bps,
            bump,
            lp_bump,
        });

        Ok(())
    }
}
