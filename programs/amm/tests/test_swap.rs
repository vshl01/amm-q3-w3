mod common;

use amm_q3_w3::error::AmmError;
use common::{assert_anchor_error, TestEnv, User, DEFAULT_FEE_BPS};

const START: u64 = 100_000_000;
const RESERVE: u64 = 1_000_000;

/// A pool seeded with `RESERVE` of each token, plus a trader holding `START` of each.
fn pool_with_liquidity(fee_bps: u16) -> (TestEnv, User) {
    let mut env = TestEnv::new(fee_bps);

    let lp = env.create_user(START, START);
    env.deposit(&lp, RESERVE, RESERVE, 0).unwrap();

    let trader = env.create_user(START, START);

    (env, trader)
}

#[test]
fn swapping_a_for_b_pays_the_curve_price_and_the_fee() {
    let (mut env, trader) = pool_with_liquidity(DEFAULT_FEE_BPS);

    // 0.30% of 100_000 = 300 to the treasury, 99_700 onto the curve.
    // out = 1_000_000 * 99_700 / (1_000_000 + 99_700) = 90_661
    env.swap(&trader, 100_000, 0, true).unwrap();

    assert_eq!(env.balance(&env.treasury_a), 300);
    assert_eq!(env.balance(&env.treasury_b), 0);

    assert_eq!(env.balance(&env.vault_a), RESERVE + 99_700);
    assert_eq!(env.balance(&env.vault_b), RESERVE - 90_661);

    assert_eq!(env.balance(&trader.token_a), START - 100_000);
    assert_eq!(env.balance(&trader.token_b), START + 90_661);
}

#[test]
fn swapping_b_for_a_takes_the_fee_on_the_b_side() {
    let (mut env, trader) = pool_with_liquidity(DEFAULT_FEE_BPS);

    env.swap(&trader, 100_000, 0, false).unwrap();

    assert_eq!(env.balance(&env.treasury_b), 300);
    assert_eq!(env.balance(&env.treasury_a), 0);

    assert_eq!(env.balance(&env.vault_b), RESERVE + 99_700);
    assert_eq!(env.balance(&env.vault_a), RESERVE - 90_661);

    assert_eq!(env.balance(&trader.token_b), START - 100_000);
    assert_eq!(env.balance(&trader.token_a), START + 90_661);
}

#[test]
fn a_swap_never_shrinks_the_invariant() {
    let (mut env, trader) = pool_with_liquidity(DEFAULT_FEE_BPS);

    let k_before = env.k();

    env.swap(&trader, 100_000, 0, true).unwrap();

    assert!(
        env.k() >= k_before,
        "k dropped from {k_before} to {}",
        env.k()
    );
}

#[test]
fn a_zero_fee_pool_collects_nothing() {
    let (mut env, trader) = pool_with_liquidity(0);

    env.swap(&trader, 100_000, 0, true).unwrap();

    assert_eq!(env.balance(&env.treasury_a), 0);
    assert_eq!(env.balance(&env.treasury_b), 0);

    // Whole input goes onto the curve.
    assert_eq!(env.balance(&env.vault_a), RESERVE + 100_000);
}

#[test]
fn fees_pile_up_across_many_swaps() {
    let (mut env, trader) = pool_with_liquidity(DEFAULT_FEE_BPS);

    for _ in 0..5 {
        env.swap(&trader, 10_000, 0, true).unwrap();
    }

    // 0.30% of 10_000 = 30, five times.
    assert_eq!(env.balance(&env.treasury_a), 150);
}

#[test]
fn a_bigger_trade_gets_a_worse_rate() {
    let (mut env, trader) = pool_with_liquidity(0);

    let before = env.balance(&trader.token_b);
    env.swap(&trader, 1_000, 0, true).unwrap();
    let small_out = env.balance(&trader.token_b) - before;

    let (mut env, trader) = pool_with_liquidity(0);

    let before = env.balance(&trader.token_b);
    env.swap(&trader, 100_000, 0, true).unwrap();
    let big_out = env.balance(&trader.token_b) - before;

    // Per unit in, the big trade must come out behind.
    assert!(big_out as u128 * 1_000 < small_out as u128 * 100_000);
}

#[test]
fn swap_respects_min_amount_out() {
    let (mut env, trader) = pool_with_liquidity(DEFAULT_FEE_BPS);

    let result = env.swap(&trader, 100_000, 90_662, true);
    assert_anchor_error(result, AmmError::SlippageExceeded);

    // Exactly the quoted amount goes through.
    env.swap(&trader, 100_000, 90_661, true).unwrap();
}

#[test]
fn a_zero_input_swap_is_rejected() {
    let (mut env, trader) = pool_with_liquidity(DEFAULT_FEE_BPS);

    assert_anchor_error(env.swap(&trader, 0, 0, true), AmmError::ZeroAmount);
}

#[test]
fn swapping_against_an_empty_pool_is_rejected() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);
    let trader = env.create_user(START, START);

    assert_anchor_error(env.swap(&trader, 1_000, 0, true), AmmError::NoLiquidity);
}

#[test]
fn a_trade_that_is_all_fee_is_rejected() {
    // At 10% fee, an input of 9 leaves 9 - 0 = 9... so use a fee that eats it all.
    let (mut env, trader) = pool_with_liquidity(amm_q3_w3::MAX_FEE_BPS);

    // 10% of 5 rounds down to 0 fee, so 5 still trades. 10_000 -> 1_000 fee.
    env.swap(&trader, 10_000, 0, true).unwrap();

    assert_eq!(env.balance(&env.treasury_a), 1_000);
    assert_eq!(env.balance(&env.vault_a), RESERVE + 9_000);
}

#[test]
fn a_round_trip_leaves_the_trader_worse_off() {
    let (mut env, trader) = pool_with_liquidity(DEFAULT_FEE_BPS);

    env.swap(&trader, 100_000, 0, true).unwrap();

    let got_b = env.balance(&trader.token_b) - START;
    env.swap(&trader, got_b, 0, false).unwrap();

    // Fees plus slippage mean you never get your token A back in full.
    assert!(env.balance(&trader.token_a) < START);
    assert_eq!(env.balance(&trader.token_b), START);
}
