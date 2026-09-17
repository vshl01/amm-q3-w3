use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("Fee is above the allowed maximum")]
    FeeTooHigh,
    #[msg("Both mints must be different")]
    SameMint,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Pool has no liquidity")]
    NoLiquidity,
    #[msg("First deposit is too small")]
    InsufficientInitialLiquidity,
    #[msg("Output is below the minimum you asked for")]
    SlippageExceeded,
    #[msg("Not enough LP tokens")]
    InsufficientLpBalance,
}
