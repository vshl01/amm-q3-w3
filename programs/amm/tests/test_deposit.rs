use anchor_lang::{
    prelude::Pubkey, solana_program::program_pack::Pack, InstructionData, ToAccountMetas,
};
use anchor_spl::token::spl_token;
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::str::FromStr;

use amm_q3_w3::{
    accounts::{Deposit, Initialize},
    POOL_SEED,
};

const PROGRAM_ID: &str = "2KRXzBtBcv9dgKDx2KGF84N8XCHdJMLMse97hzYFpNMF";
const MINT_LEN: usize = 82;
const TOKEN_ACCOUNT_LEN: usize = 165;

const USER_START: u64 = 1_000_000_000;
const DEPOSIT_A: u64 = 300_000_000;
const DEPOSIT_B: u64 = 700_000_000;

#[test]
fn test_deposit() {
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

    // 3. Create + fund authority
    let authority = Keypair::new();

    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    // 4. Create Token A mint
    let mint_a = Keypair::new();

    let rent_mint = svm.minimum_balance_for_rent_exemption(MINT_LEN);

    let create_mint_a = solana_system_interface::instruction::create_account(
        &authority.pubkey(),
        &mint_a.pubkey(),
        rent_mint,
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

    // 5. Create Token B mint
    let mint_b = Keypair::new();

    let create_mint_b = solana_system_interface::instruction::create_account(
        &authority.pubkey(),
        &mint_b.pubkey(),
        rent_mint,
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

    // 6. Send both mint creations
    let tx = Transaction::new(
        &[&authority, &mint_a, &mint_b],
        Message::new(
            &[create_mint_a, init_mint_a, create_mint_b, init_mint_b],
            Some(&authority.pubkey()),
        ),
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // 7. Derive Pool PDA
    let (pool, _bump) = Pubkey::find_program_address(
        &[
            POOL_SEED,
            mint_a.pubkey().as_ref(),
            mint_b.pubkey().as_ref(),
        ],
        &program_id,
    );

    // 8. Run initialize so pool + vaults exist
    let vault_a = Keypair::new();
    let vault_b = Keypair::new();

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

    let tx = Transaction::new(
        &[&authority, &vault_a, &vault_b],
        Message::new(&[initialize_ix], Some(&authority.pubkey())),
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // 9. Create the user's two token accounts
    let user_token_a = Keypair::new();
    let user_token_b = Keypair::new();

    let rent_token = svm.minimum_balance_for_rent_exemption(TOKEN_ACCOUNT_LEN);

    let create_user_a = solana_system_interface::instruction::create_account(
        &authority.pubkey(),
        &user_token_a.pubkey(),
        rent_token,
        TOKEN_ACCOUNT_LEN as u64,
        &spl_token::ID,
    );

    let init_user_a = spl_token::instruction::initialize_account3(
        &spl_token::ID,
        &user_token_a.pubkey(),
        &mint_a.pubkey(),
        &authority.pubkey(),
    )
    .unwrap();

    let create_user_b = solana_system_interface::instruction::create_account(
        &authority.pubkey(),
        &user_token_b.pubkey(),
        rent_token,
        TOKEN_ACCOUNT_LEN as u64,
        &spl_token::ID,
    );

    let init_user_b = spl_token::instruction::initialize_account3(
        &spl_token::ID,
        &user_token_b.pubkey(),
        &mint_b.pubkey(),
        &authority.pubkey(),
    )
    .unwrap();

    // 10. Mint starting balance into both user accounts
    let mint_to_a = spl_token::instruction::mint_to(
        &spl_token::ID,
        &mint_a.pubkey(),
        &user_token_a.pubkey(),
        &authority.pubkey(),
        &[],
        USER_START,
    )
    .unwrap();

    let mint_to_b = spl_token::instruction::mint_to(
        &spl_token::ID,
        &mint_b.pubkey(),
        &user_token_b.pubkey(),
        &authority.pubkey(),
        &[],
        USER_START,
    )
    .unwrap();

    let tx = Transaction::new(
        &[&authority, &user_token_a, &user_token_b],
        Message::new(
            &[
                create_user_a,
                init_user_a,
                create_user_b,
                init_user_b,
                mint_to_a,
                mint_to_b,
            ],
            Some(&authority.pubkey()),
        ),
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // 11. Build deposit instruction
    let deposit_ix = anchor_lang::solana_program::instruction::Instruction {
        program_id,
        accounts: Deposit {
            authority: authority.pubkey(),
            pool,
            user_token_a: user_token_a.pubkey(),
            user_token_b: user_token_b.pubkey(),
            vault_a: vault_a.pubkey(),
            vault_b: vault_b.pubkey(),
            token_program: spl_token::ID,
        }
        .to_account_metas(None),
        data: amm_q3_w3::instruction::Deposit {
            amount_a: DEPOSIT_A,
            amount_b: DEPOSIT_B,
        }
        .data(),
    };

    // 12. Send deposit transaction
    let tx = Transaction::new(
        &[&authority],
        Message::new(&[deposit_ix], Some(&authority.pubkey())),
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // 13. Vaults should now hold the deposited amounts
    let vault_a_state = read_token_account(&svm, &vault_a.pubkey());
    let vault_b_state = read_token_account(&svm, &vault_b.pubkey());

    assert_eq!(vault_a_state.amount, DEPOSIT_A);
    assert_eq!(vault_a_state.owner, pool);

    assert_eq!(vault_b_state.amount, DEPOSIT_B);
    assert_eq!(vault_b_state.owner, pool);

    // 14. User should be short by exactly what was deposited
    let user_a_state = read_token_account(&svm, &user_token_a.pubkey());
    let user_b_state = read_token_account(&svm, &user_token_b.pubkey());

    assert_eq!(user_a_state.amount, USER_START - DEPOSIT_A);
    assert_eq!(user_b_state.amount, USER_START - DEPOSIT_B);

    println!("Deposit test passed ✅");
}

fn read_token_account(svm: &LiteSVM, key: &Pubkey) -> spl_token::state::Account {
    let account = svm.get_account(key).expect("token account not found");

    spl_token::state::Account::unpack(&account.data).unwrap()
}
