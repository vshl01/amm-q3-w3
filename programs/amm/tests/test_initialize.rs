use anchor_lang::{
    prelude::Pubkey, solana_program::program_pack::Pack, AccountDeserialize, InstructionData,
    ToAccountMetas,
};
use anchor_spl::token::spl_token;
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::str::FromStr;

use amm_q3_w3::{accounts::Initialize, state::Pool, POOL_SEED};

const PROGRAM_ID: &str = "2KRXzBtBcv9dgKDx2KGF84N8XCHdJMLMse97hzYFpNMF";
const MINT_LEN: usize = 82;

#[test]
fn test_initialize() {
    // 1. Start LiteSVM
    let mut svm = LiteSVM::new();

    // 2. Load compiled AMM program
    let program = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/deploy/amm_q3_w3.so"
    ))
    .unwrap();

    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();

    svm.add_program(program_id, &program).unwrap();

    // 3. Create authority
    let authority = Keypair::new();

    // 4. Fund authority
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    // 5. Create Token A mint
    let mint_a = Keypair::new();

    let rent_a = svm.minimum_balance_for_rent_exemption(MINT_LEN);

    let create_mint_a = solana_system_interface::instruction::create_account(
        &authority.pubkey(),
        &mint_a.pubkey(),
        rent_a,
        MINT_LEN as u64,
        &spl_token::ID,
    );

    let init_mint_a = spl_token::instruction::initialize_mint2(
        &spl_token::ID,
        &mint_a.pubkey(),
        &authority.pubkey(),
        None,
        6,
    )
    .unwrap();

    // 6. Create Token B mint
    let mint_b = Keypair::new();

    let rent_b = svm.minimum_balance_for_rent_exemption(MINT_LEN);

    let create_mint_b = solana_system_interface::instruction::create_account(
        &authority.pubkey(),
        &mint_b.pubkey(),
        rent_b,
        MINT_LEN as u64,
        &spl_token::ID,
    );

    let init_mint_b = spl_token::instruction::initialize_mint2(
        &spl_token::ID,
        &mint_b.pubkey(),
        &authority.pubkey(),
        None,
        6,
    )
    .unwrap();

    // 7. Create both mints
    let tx = Transaction::new(
        &[&authority, &mint_a, &mint_b],
        Message::new(
            &[create_mint_a, init_mint_a, create_mint_b, init_mint_b],
            Some(&authority.pubkey()),
        ),
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // 8. Derive Pool PDA
    let (pool, expected_bump) = Pubkey::find_program_address(
        &[
            POOL_SEED,
            mint_a.pubkey().as_ref(),
            mint_b.pubkey().as_ref(),
        ],
        &program_id,
    );

    // 9. Create token vault accounts
    let vault_a = Keypair::new();
    let vault_b = Keypair::new();

    // 10. Build initialize instruction
    let initialize_ix = anchor_lang::solana_program::instruction::Instruction {
        program_id,
        accounts: Initialize {
            authority: authority.pubkey(),
            mint_a: mint_a.pubkey(),
            mint_b: mint_b.pubkey(),
            pool,
            vault_a: vault_a.pubkey(),
            vault_b: vault_b.pubkey(),
            system_program: anchor_lang::solana_program::system_program::ID,
            token_program: spl_token::ID,
        }
        .to_account_metas(None),
        data: amm_q3_w3::instruction::Initialize {}.data(),
    };

    // 11. Send initialize transaction
    let tx = Transaction::new(
        &[&authority, &vault_a, &vault_b],
        Message::new(&[initialize_ix], Some(&authority.pubkey())),
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // 12. Read Pool account
    let pool_account = svm.get_account(&pool).expect("Pool was not created");

    assert_eq!(pool_account.owner, program_id);

    // 13. Deserialize Pool state
    let pool_state = Pool::try_deserialize(&mut pool_account.data.as_slice()).unwrap();

    // 14. Verify Pool fields
    assert_eq!(pool_state.mint_a, mint_a.pubkey());
    assert_eq!(pool_state.mint_b, mint_b.pubkey());
    assert_eq!(pool_state.vault_a, vault_a.pubkey());
    assert_eq!(pool_state.vault_b, vault_b.pubkey());
    assert_eq!(pool_state.bump, expected_bump);

    // 15. Verify Vault A
    let vault_a_account = svm
        .get_account(&vault_a.pubkey())
        .expect("Vault A was not created");

    let vault_a_state = spl_token::state::Account::unpack(&vault_a_account.data).unwrap();

    assert_eq!(vault_a_state.mint, mint_a.pubkey());
    assert_eq!(vault_a_state.owner, pool);

    // 16. Verify Vault B
    let vault_b_account = svm
        .get_account(&vault_b.pubkey())
        .expect("Vault B was not created");

    let vault_b_state = spl_token::state::Account::unpack(&vault_b_account.data).unwrap();

    assert_eq!(vault_b_state.mint, mint_b.pubkey());
    assert_eq!(vault_b_state.owner, pool);

    println!("Initialize test passed ✅");
}
