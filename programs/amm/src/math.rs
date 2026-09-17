//! Constant-product market maker math, written by hand (no curve library).
//!
//! The invariant is `reserve_a * reserve_b = k`. Every function below works in
//! `u128` so the intermediate products cannot overflow, then narrows back to
//! `u64` at the end.

use anchor_lang::prelude::*;

use crate::{error::AmmError, BPS_DENOMINATOR};

/// Integer square root (Babylonian / Newton method).
///
/// Returns `floor(sqrt(n))`.
pub fn integer_sqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }

    // Start high enough that the iteration only ever walks downward.
    let mut x = n;
    let mut y = n.div_ceil(2);

    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }

    x
}

/// LP tokens minted for the very first deposit: `sqrt(amount_a * amount_b)`.
///
/// `MINIMUM_LIQUIDITY` is burned by the caller, so this must be larger than it.
pub fn initial_lp_tokens(amount_a: u64, amount_b: u64) -> Result<u64> {
    let product = (amount_a as u128)
        .checked_mul(amount_b as u128)
        .ok_or(AmmError::Overflow)?;

    u64::try_from(integer_sqrt(product)).map_err(|_| AmmError::Overflow.into())
}

/// LP tokens minted for a later deposit.
///
/// The depositor is credited for whichever side is proportionally smaller, so
/// nobody can mint extra LP by sending a lopsided pair.
pub fn lp_tokens_for_deposit(
    amount_a: u64,
    amount_b: u64,
    reserve_a: u64,
    reserve_b: u64,
    lp_supply: u64,
) -> Result<u64> {
    require!(reserve_a > 0 && reserve_b > 0, AmmError::NoLiquidity);

    let from_a = mul_div(amount_a as u128, lp_supply as u128, reserve_a as u128)?;
    let from_b = mul_div(amount_b as u128, lp_supply as u128, reserve_b as u128)?;

    u64::try_from(from_a.min(from_b)).map_err(|_| AmmError::Overflow.into())
}

/// Tokens returned for burning `lp_amount` LP: `lp_amount / lp_supply` of the vault.
pub fn amount_for_lp(lp_amount: u64, reserve: u64, lp_supply: u64) -> Result<u64> {
    require!(lp_supply > 0, AmmError::NoLiquidity);

    let amount = mul_div(lp_amount as u128, reserve as u128, lp_supply as u128)?;

    u64::try_from(amount).map_err(|_| AmmError::Overflow.into())
}

/// Fee skimmed off the input side of a swap, in basis points.
pub fn fee_amount(amount_in: u64, fee_bps: u16) -> Result<u64> {
    let fee = mul_div(amount_in as u128, fee_bps as u128, BPS_DENOMINATOR)?;

    u64::try_from(fee).map_err(|_| AmmError::Overflow.into())
}

/// Constant-product swap output.
///
/// ```text
/// (reserve_in + amount_in) * (reserve_out - amount_out) >= reserve_in * reserve_out
///
/// amount_out = reserve_out * amount_in / (reserve_in + amount_in)
/// ```
///
/// `amount_in` here is already net of the fee, so the fee leaves the curve
/// untouched and simply goes to the treasury.
pub fn swap_output(amount_in: u64, reserve_in: u64, reserve_out: u64) -> Result<u64> {
    require!(reserve_in > 0 && reserve_out > 0, AmmError::NoLiquidity);
    require!(amount_in > 0, AmmError::ZeroAmount);

    let numerator = (reserve_out as u128)
        .checked_mul(amount_in as u128)
        .ok_or(AmmError::Overflow)?;

    let denominator = (reserve_in as u128)
        .checked_add(amount_in as u128)
        .ok_or(AmmError::Overflow)?;

    let amount_out = numerator / denominator;

    // Division floors, so the pool always keeps the rounding dust and `k` can
    // only grow. Still, never hand out the whole other side.
    require!(amount_out < reserve_out as u128, AmmError::NoLiquidity);

    u64::try_from(amount_out).map_err(|_| AmmError::Overflow.into())
}

/// `a * b / c` in u128, erroring instead of wrapping or dividing by zero.
fn mul_div(a: u128, b: u128, c: u128) -> Result<u128> {
    require!(c > 0, AmmError::NoLiquidity);

    a.checked_mul(b)
        .ok_or(AmmError::Overflow)?
        .checked_div(c)
        .ok_or(AmmError::Overflow.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrt_matches_known_values() {
        assert_eq!(integer_sqrt(0), 0);
        assert_eq!(integer_sqrt(1), 1);
        assert_eq!(integer_sqrt(4), 2);
        assert_eq!(integer_sqrt(15), 3);
        assert_eq!(integer_sqrt(16), 4);
        assert_eq!(integer_sqrt(1_000_000), 1_000);
        assert_eq!(integer_sqrt(u64::MAX as u128), 4_294_967_295);
    }

    #[test]
    fn first_deposit_is_geometric_mean() {
        assert_eq!(initial_lp_tokens(1_000, 1_000).unwrap(), 1_000);
        assert_eq!(initial_lp_tokens(400, 900).unwrap(), 600);
    }

    #[test]
    fn later_deposit_uses_the_smaller_side() {
        // Pool is 1000/1000 with 1000 LP out. A balanced 100/100 deposit earns 100.
        assert_eq!(
            lp_tokens_for_deposit(100, 100, 1_000, 1_000, 1_000).unwrap(),
            100
        );

        // Sending extra B earns nothing more - the A side caps it.
        assert_eq!(
            lp_tokens_for_deposit(100, 500, 1_000, 1_000, 1_000).unwrap(),
            100
        );
    }

    #[test]
    fn swap_keeps_the_product_from_shrinking() {
        let (reserve_in, reserve_out) = (1_000_000u64, 1_000_000u64);
        let amount_in = 10_000u64;

        let amount_out = swap_output(amount_in, reserve_in, reserve_out).unwrap();

        let k_before = reserve_in as u128 * reserve_out as u128;
        let k_after = (reserve_in + amount_in) as u128 * (reserve_out - amount_out) as u128;

        assert!(k_after >= k_before);
    }

    #[test]
    fn bigger_trades_get_a_worse_price() {
        let small = swap_output(1_000, 1_000_000, 1_000_000).unwrap();
        let big = swap_output(100_000, 1_000_000, 1_000_000).unwrap();

        // Price per unit in, scaled to compare.
        assert!(big as u128 * 1_000 < small as u128 * 100_000);
    }

    #[test]
    fn fee_is_basis_points_of_the_input() {
        assert_eq!(fee_amount(1_000_000, 30).unwrap(), 3_000); // 0.30%
        assert_eq!(fee_amount(1_000_000, 0).unwrap(), 0);
    }

    #[test]
    fn empty_pool_is_rejected() {
        assert!(swap_output(100, 0, 1_000).is_err());
        assert!(amount_for_lp(100, 1_000, 0).is_err());
    }
}
