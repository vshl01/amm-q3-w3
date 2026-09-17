use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Pool {
    /// The two tokens this pool trades.
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,

    /// Where the pool's liquidity actually sits. Owned by the pool PDA.
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,

    /// LP mint. PDA of this pool, mint authority is the pool PDA.
    pub lp_mint: Pubkey,

    /// Where swap fees are collected. Owned by `authority`, not the pool, so
    /// the protocol can spend them with a plain SPL transfer.
    pub treasury_a: Pubkey,
    pub treasury_b: Pubkey,

    /// Who created the pool and owns the treasuries.
    pub authority: Pubkey,

    /// Swap fee in basis points, taken off the input side.
    pub fee_bps: u16,

    pub bump: u8,
    pub lp_bump: u8,
}
