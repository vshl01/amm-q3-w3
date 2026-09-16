mod common;

use amm_q3_w3::error::AmmError;
use common::{assert_anchor_error, TestEnv, DEFAULT_FEE_BPS};

const START: u64 = 100_000_000;

#[test]
fn first_deposit_mints_the_geometric_mean_and_funds_the_vaults() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);
    let lp = env.create_user(START, START);

    // sqrt(1_000_000 * 4_000_000) = 2_000_000
    env.deposit(&lp, 1_000_000, 4_000_000, 0).unwrap();

    assert_eq!(env.balance(&env.vault_a), 1_000_000);
    assert_eq!(env.balance(&env.vault_b), 4_000_000);

    assert_eq!(env.balance(&lp.token_a), START - 1_000_000);
    assert_eq!(env.balance(&lp.token_b), START - 4_000_000);

    assert_eq!(env.balance(&lp.lp), 2_000_000);
    assert_eq!(env.lp_supply(), 2_000_000);
}

#[test]
fn second_deposit_is_credited_in_proportion() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);

    let first = env.create_user(START, START);
    env.deposit(&first, 1_000_000, 4_000_000, 0).unwrap();

    // Half the pool's size at the same ratio earns half the existing supply.
    let second = env.create_user(START, START);
    env.deposit(&second, 500_000, 2_000_000, 0).unwrap();

    assert_eq!(env.balance(&second.lp), 1_000_000);
    assert_eq!(env.lp_supply(), 3_000_000);

    assert_eq!(env.balance(&env.vault_a), 1_500_000);
    assert_eq!(env.balance(&env.vault_b), 6_000_000);
}

#[test]
fn a_lopsided_deposit_is_credited_on_the_smaller_side_only() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);

    let first = env.create_user(START, START);
    env.deposit(&first, 1_000_000, 4_000_000, 0).unwrap();

    // Ratio asks for 500_000 A against 2_000_000 B. Sending 4_000_000 B is
    // twice as much as needed, so the A side caps the LP minted.
    let second = env.create_user(START, START);
    env.deposit(&second, 500_000, 4_000_000, 0).unwrap();

    assert_eq!(env.balance(&second.lp), 1_000_000);

    // The extra B is still transferred - it becomes a gift to every LP.
    assert_eq!(env.balance(&env.vault_b), 8_000_000);
}

#[test]
fn deposit_respects_min_lp() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);
    let lp = env.create_user(START, START);

    // Asking for more LP than the maths can give must fail.
    let result = env.deposit(&lp, 1_000_000, 4_000_000, 2_000_001);
    assert_anchor_error(result, AmmError::SlippageExceeded);

    // Exactly the right number is fine.
    env.deposit(&lp, 1_000_000, 4_000_000, 2_000_000).unwrap();
}

#[test]
fn zero_amounts_are_rejected() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);
    let lp = env.create_user(START, START);

    assert_anchor_error(env.deposit(&lp, 0, 1_000, 0), AmmError::ZeroAmount);
    assert_anchor_error(env.deposit(&lp, 1_000, 0, 0), AmmError::ZeroAmount);
}

#[test]
fn a_dust_first_deposit_is_rejected() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);
    let lp = env.create_user(START, START);

    // sqrt(10 * 10) = 10, well under MINIMUM_LIQUIDITY.
    let result = env.deposit(&lp, 10, 10, 0);

    assert_anchor_error(result, AmmError::InsufficientInitialLiquidity);
}

#[test]
fn a_deposit_too_small_to_earn_lp_is_rejected() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);

    // Lopsided pool: 1_000_000 A / 16_000_000 B, so supply is 4_000_000 LP and
    // the B side is worth 1 LP per 4 tokens.
    let first = env.create_user(START, START);
    env.deposit(&first, 1_000_000, 16_000_000, 0).unwrap();
    assert_eq!(env.lp_supply(), 4_000_000);

    // 3 units of B is worth 3/4 of an LP, which rounds down to nothing.
    let second = env.create_user(START, START);
    let result = env.deposit(&second, 1, 3, 0);

    assert_anchor_error(result, AmmError::ZeroAmount);
}

#[test]
fn deposits_never_shrink_the_invariant() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);

    let first = env.create_user(START, START);
    env.deposit(&first, 1_000_000, 4_000_000, 0).unwrap();
    let k_after_first = env.k();

    let second = env.create_user(START, START);
    env.deposit(&second, 500_000, 2_000_000, 0).unwrap();

    assert!(env.k() > k_after_first);
}
