mod common;

use anchor_lang::solana_program::program_pack::Pack;
use anchor_spl::token::spl_token;
use common::{TestEnv, DEFAULT_FEE_BPS};
use solana_signer::Signer;

#[test]
fn initialize_records_every_account_on_the_pool() {
    let env = TestEnv::new(DEFAULT_FEE_BPS);

    // Pool account belongs to the program and stores everything later
    // instructions will need to validate against.
    let pool_account = env.svm.get_account(&env.pool).unwrap();
    assert_eq!(pool_account.owner, env.program_id);

    let pool = env.pool_state();

    assert_eq!(pool.mint_a, env.mint_a);
    assert_eq!(pool.mint_b, env.mint_b);
    assert_eq!(pool.vault_a, env.vault_a);
    assert_eq!(pool.vault_b, env.vault_b);
    assert_eq!(pool.lp_mint, env.lp_mint);
    assert_eq!(pool.treasury_a, env.treasury_a);
    assert_eq!(pool.treasury_b, env.treasury_b);
    assert_eq!(pool.authority, env.authority.pubkey());
    assert_eq!(pool.fee_bps, DEFAULT_FEE_BPS);
    assert_eq!(pool.bump, env.pool_bump);
    assert_eq!(pool.lp_bump, env.lp_bump);
}

#[test]
fn vaults_start_empty_and_are_owned_by_the_pool() {
    let env = TestEnv::new(DEFAULT_FEE_BPS);

    let vault_a = env.token_account(&env.vault_a);
    let vault_b = env.token_account(&env.vault_b);

    assert_eq!(vault_a.mint, env.mint_a);
    assert_eq!(vault_a.owner, env.pool);
    assert_eq!(vault_a.amount, 0);

    assert_eq!(vault_b.mint, env.mint_b);
    assert_eq!(vault_b.owner, env.pool);
    assert_eq!(vault_b.amount, 0);
}

#[test]
fn treasuries_start_empty_and_are_owned_by_the_authority() {
    let env = TestEnv::new(DEFAULT_FEE_BPS);

    let treasury_a = env.token_account(&env.treasury_a);
    let treasury_b = env.token_account(&env.treasury_b);

    assert_eq!(treasury_a.mint, env.mint_a);
    assert_eq!(treasury_a.owner, env.authority.pubkey());
    assert_eq!(treasury_a.amount, 0);

    assert_eq!(treasury_b.mint, env.mint_b);
    assert_eq!(treasury_b.owner, env.authority.pubkey());
    assert_eq!(treasury_b.amount, 0);
}

#[test]
fn lp_mint_starts_empty_with_the_pool_as_authority() {
    let env = TestEnv::new(DEFAULT_FEE_BPS);

    let account = env.svm.get_account(&env.lp_mint).unwrap();
    let mint = spl_token::state::Mint::unpack(&account.data).unwrap();

    assert_eq!(mint.supply, 0);
    assert_eq!(mint.decimals, 6);
    assert_eq!(mint.mint_authority.unwrap(), env.pool);

    // Nobody but the program can mint LP.
    assert_ne!(mint.mint_authority.unwrap(), env.authority.pubkey());
}

#[test]
fn a_zero_fee_pool_is_allowed() {
    let env = TestEnv::new(0);

    assert_eq!(env.pool_state().fee_bps, 0);
}

#[test]
fn fee_above_the_cap_is_rejected() {
    let result = TestEnv::try_new(amm_q3_w3::MAX_FEE_BPS + 1);

    assert!(
        result.is_err(),
        "initialize should reject a fee above MAX_FEE_BPS"
    );
}

#[test]
fn the_max_fee_itself_is_allowed() {
    let env = TestEnv::new(amm_q3_w3::MAX_FEE_BPS);

    assert_eq!(env.pool_state().fee_bps, amm_q3_w3::MAX_FEE_BPS);
}
