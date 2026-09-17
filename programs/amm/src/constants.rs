use anchor_lang::prelude::*;

#[constant]
pub const POOL_SEED: &[u8] = b"pool";

#[constant]
pub const LP_MINT_SEED: &[u8] = b"lp";

/// Basis-point denominator. 10_000 bps = 100%.
pub const BPS_DENOMINATOR: u128 = 10_000;

/// Hard cap on the swap fee so a pool can never be created with a silly fee.
/// 1_000 bps = 10%.
pub const MAX_FEE_BPS: u16 = 1_000;

/// The first deposit must mint more LP than this. It stops a pool from being
/// opened with dust, which is what makes LP share price manipulable.
pub const MINIMUM_LIQUIDITY: u64 = 1_000;

/// Decimals used by every LP mint.
pub const LP_DECIMALS: u8 = 6;
