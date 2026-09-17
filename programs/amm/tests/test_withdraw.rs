mod common;

use amm_q3_w3::error::AmmError;
use common::{assert_anchor_error, TestEnv, DEFAULT_FEE_BPS};

const START: u64 = 100_000_000;

#[test]
fn withdrawing_everything_empties_the_pool() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);

    let lp = env.create_user(START, START);
    env.deposit(&lp, 1_000_000, 4_000_000, 0).unwrap();

    env.withdraw(&lp, 2_000_000, 0, 0).unwrap();

    assert_eq!(env.balance(&env.vault_a), 0);
    assert_eq!(env.balance(&env.vault_b), 0);
    assert_eq!(env.lp_supply(), 0);
    assert_eq!(env.balance(&lp.lp), 0);

    // The only LP gets every token back.
    assert_eq!(env.balance(&lp.token_a), START);
    assert_eq!(env.balance(&lp.token_b), START);
}

#[test]
fn a_partial_withdraw_pays_out_in_proportion() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);

    let lp = env.create_user(START, START);
    env.deposit(&lp, 1_000_000, 4_000_000, 0).unwrap();

    // Half the LP supply is half of each vault.
    env.withdraw(&lp, 1_000_000, 0, 0).unwrap();

    assert_eq!(env.balance(&env.vault_a), 500_000);
    assert_eq!(env.balance(&env.vault_b), 2_000_000);

    assert_eq!(env.lp_supply(), 1_000_000);
    assert_eq!(env.balance(&lp.lp), 1_000_000);

    assert_eq!(env.balance(&lp.token_a), START - 500_000);
    assert_eq!(env.balance(&lp.token_b), START - 2_000_000);
}

#[test]
fn two_providers_split_the_pool_by_share() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);

    let first = env.create_user(START, START);
    env.deposit(&first, 1_000_000, 1_000_000, 0).unwrap();

    // Second LP doubles the pool, so each ends up holding half the supply.
    let second = env.create_user(START, START);
    env.deposit(&second, 1_000_000, 1_000_000, 0).unwrap();

    assert_eq!(env.balance(&first.lp), 1_000_000);
    assert_eq!(env.balance(&second.lp), 1_000_000);

    env.withdraw(&second, 1_000_000, 0, 0).unwrap();

    // Second LP gets back exactly what they put in.
    assert_eq!(env.balance(&second.token_a), START);
    assert_eq!(env.balance(&second.token_b), START);

    assert_eq!(env.balance(&env.vault_a), 1_000_000);
    assert_eq!(env.balance(&env.vault_b), 1_000_000);
}

#[test]
fn a_provider_keeps_the_slippage_left_behind_by_traders() {
    let mut env = TestEnv::new(0);

    let lp = env.create_user(START, START);
    env.deposit(&lp, 1_000_000, 1_000_000, 0).unwrap();

    // A trader round-trips, leaving rounding dust in the vaults.
    let trader = env.create_user(START, START);
    env.swap(&trader, 100_000, 0, true).unwrap();

    let got_b = env.balance(&trader.token_b) - START;
    env.swap(&trader, got_b, 0, false).unwrap();

    env.withdraw(&lp, 1_000_000, 0, 0).unwrap();

    let total_out = env.balance(&lp.token_a) + env.balance(&lp.token_b);

    assert!(
        total_out >= 2 * START,
        "LP should never be worse off after trades: {total_out}"
    );
}

#[test]
fn withdraw_respects_min_a_and_min_b() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);

    let lp = env.create_user(START, START);
    env.deposit(&lp, 1_000_000, 4_000_000, 0).unwrap();

    // Half the supply is worth 500_000 A / 2_000_000 B.
    assert_anchor_error(
        env.withdraw(&lp, 1_000_000, 500_001, 0),
        AmmError::SlippageExceeded,
    );

    assert_anchor_error(
        env.withdraw(&lp, 1_000_000, 0, 2_000_001),
        AmmError::SlippageExceeded,
    );

    env.withdraw(&lp, 1_000_000, 500_000, 2_000_000).unwrap();
}

#[test]
fn withdrawing_zero_is_rejected() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);

    let lp = env.create_user(START, START);
    env.deposit(&lp, 1_000_000, 4_000_000, 0).unwrap();

    assert_anchor_error(env.withdraw(&lp, 0, 0, 0), AmmError::ZeroAmount);
}

#[test]
fn withdrawing_more_lp_than_you_hold_is_rejected() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);

    let lp = env.create_user(START, START);
    env.deposit(&lp, 1_000_000, 4_000_000, 0).unwrap();

    assert_anchor_error(
        env.withdraw(&lp, 2_000_001, 0, 0),
        AmmError::InsufficientLpBalance,
    );
}

#[test]
fn someone_with_no_lp_tokens_cannot_withdraw() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);

    let lp = env.create_user(START, START);
    env.deposit(&lp, 1_000_000, 4_000_000, 0).unwrap();

    let outsider = env.create_user(START, START);

    assert_anchor_error(
        env.withdraw(&outsider, 1, 0, 0),
        AmmError::InsufficientLpBalance,
    );
}

#[test]
fn the_pool_can_be_reseeded_after_a_full_withdraw() {
    let mut env = TestEnv::new(DEFAULT_FEE_BPS);

    let lp = env.create_user(START, START);
    env.deposit(&lp, 1_000_000, 4_000_000, 0).unwrap();
    env.withdraw(&lp, 2_000_000, 0, 0).unwrap();

    // Supply is back to zero, so this counts as a first deposit again.
    env.deposit(&lp, 2_000_000, 2_000_000, 0).unwrap();

    assert_eq!(env.lp_supply(), 2_000_000);
    assert_eq!(env.balance(&env.vault_a), 2_000_000);
}
