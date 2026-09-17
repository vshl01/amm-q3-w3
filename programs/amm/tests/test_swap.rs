use anchor_lang::{
    prelude::Pubkey,
    AccountDeserialize,
    InstructionData,
    ToAccountMetas,
};
use anchor_lang::solana_program::program_pack::Pack;
use anchor_spl::token::spl_token;
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::str::FromStr;

use amm_q3_w3::{
    accounts::{Deposit, Initialize, Swap},
    instruction,
    state::Pool,
    POOL_SEED,
};

const PROGRAM_ID: &str = "2KRXzBtBcv9dgKDx2KGF84N8XCHdJMLMse97hzYFpNMF";
const MINT_LEN: usize = 82;

fn token_balance(svm: &LiteSVM, account: &Pubkey) -> u64 {
    let account_data = svm.get_account(account).unwrap();

    let token_account =
        spl_token::state::Account::unpack(&account_data.data).unwrap();

    token_account.amount
}

#[test]
fn test_swap() {
    let mut svm = LiteSVM::new();

    // Load program
    let program = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/deploy/amm_q3_w3.so"
    ))
    .unwrap();

    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();

    svm.add_program(program_id, &program).unwrap();

    // Authority
    let authority = Keypair::new();

    svm.airdrop(&authority.pubkey(), 10_000_000_000)
        .unwrap();

    // --------------------------------------------------
    // Create Token A mint
    // --------------------------------------------------

    let mint_a = Keypair::new();

    let create_mint_a = solana_system_interface::instruction::create_account(
        &authority.pubkey(),
        &mint_a.pubkey(),
        1_000_000_000,
        MINT_LEN as u64,
        &spl_token::id(),
    );

    let init_mint_a = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_a.pubkey(),
        &authority.pubkey(),
        None,
        6,
    )
    .unwrap();

    let message = Message::new(
        &[create_mint_a, init_mint_a],
        Some(&authority.pubkey()),
    );

    let tx = Transaction::new(
        &[&authority, &mint_a],
        message,
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // --------------------------------------------------
    // Create Token B mint
    // --------------------------------------------------

    let mint_b = Keypair::new();

    let create_mint_b = solana_system_interface::instruction::create_account(
        &authority.pubkey(),
        &mint_b.pubkey(),
        1_000_000_000,
        MINT_LEN as u64,
        &spl_token::id(),
    );

    let init_mint_b = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_b.pubkey(),
        &authority.pubkey(),
        None,
        6,
    )
    .unwrap();

    let message = Message::new(
        &[create_mint_b, init_mint_b],
        Some(&authority.pubkey()),
    );

    let tx = Transaction::new(
        &[&authority, &mint_b],
        message,
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // --------------------------------------------------
    // Initialize AMM
    // --------------------------------------------------

    let (pool, _) = Pubkey::find_program_address(
        &[
            POOL_SEED,
            mint_a.pubkey().as_ref(),
            mint_b.pubkey().as_ref(),
        ],
        &program_id,
    );

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
            token_program: spl_token::id(),
        }
        .to_account_metas(None),
        data: instruction::Initialize {}.data(),
    };

    let message = Message::new(
        &[initialize_ix],
        Some(&authority.pubkey()),
    );

    let tx = Transaction::new(
        &[&authority, &vault_a, &vault_b],
        message,
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // --------------------------------------------------
    // Create user Token A account
    // --------------------------------------------------

    let user_token_a = Keypair::new();

    let create_user_a =
        solana_system_interface::instruction::create_account(
            &authority.pubkey(),
            &user_token_a.pubkey(),
            1_000_000_000,
            spl_token::state::Account::LEN as u64,
            &spl_token::id(),
        );

    let init_user_a = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &user_token_a.pubkey(),
        &mint_a.pubkey(),
        &authority.pubkey(),
    )
    .unwrap();

    let message = Message::new(
        &[create_user_a, init_user_a],
        Some(&authority.pubkey()),
    );

    let tx = Transaction::new(
        &[&authority, &user_token_a],
        message,
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // --------------------------------------------------
    // Create user Token B account
    // --------------------------------------------------

    let user_token_b = Keypair::new();

    let create_user_b =
        solana_system_interface::instruction::create_account(
            &authority.pubkey(),
            &user_token_b.pubkey(),
            1_000_000_000,
            spl_token::state::Account::LEN as u64,
            &spl_token::id(),
        );

    let init_user_b = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &user_token_b.pubkey(),
        &mint_b.pubkey(),
        &authority.pubkey(),
    )
    .unwrap();

    let message = Message::new(
        &[create_user_b, init_user_b],
        Some(&authority.pubkey()),
    );

    let tx = Transaction::new(
        &[&authority, &user_token_b],
        message,
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // --------------------------------------------------
    // Mint 1000 A and 1000 B to user
    // --------------------------------------------------

    let mint_a_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint_a.pubkey(),
        &user_token_a.pubkey(),
        &authority.pubkey(),
        &[],
        1000,
    )
    .unwrap();

    let mint_b_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint_b.pubkey(),
        &user_token_b.pubkey(),
        &authority.pubkey(),
        &[],
        1000,
    )
    .unwrap();

    let message = Message::new(
        &[mint_a_ix, mint_b_ix],
        Some(&authority.pubkey()),
    );

    let tx = Transaction::new(
        &[&authority],
        message,
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // --------------------------------------------------
    // Deposit 500 A + 500 B
    // --------------------------------------------------

    let deposit_ix = anchor_lang::solana_program::instruction::Instruction {
        program_id,
        accounts: Deposit {
            authority: authority.pubkey(),
            pool,
            user_token_a: user_token_a.pubkey(),
            user_token_b: user_token_b.pubkey(),
            vault_a: vault_a.pubkey(),
            vault_b: vault_b.pubkey(),
            token_program: spl_token::id(),
        }
        .to_account_metas(None),
        data: instruction::Deposit {
            amount_a: 500,
            amount_b: 500,
        }
        .data(),
    };

    let message = Message::new(
        &[deposit_ix],
        Some(&authority.pubkey()),
    );

    let tx = Transaction::new(
        &[&authority],
        message,
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // Verify initial swap state
    assert_eq!(token_balance(&svm, &vault_a.pubkey()), 500);
    assert_eq!(token_balance(&svm, &vault_b.pubkey()), 500);

    assert_eq!(token_balance(&svm, &user_token_a.pubkey()), 500);
    assert_eq!(token_balance(&svm, &user_token_b.pubkey()), 500);

    // --------------------------------------------------
    // SWAP: User gives 100 A
    // --------------------------------------------------

    let swap_ix = anchor_lang::solana_program::instruction::Instruction {
        program_id,
        accounts: Swap {
            authority: authority.pubkey(),
            pool,
            user_token_a: user_token_a.pubkey(),
            user_token_b: user_token_b.pubkey(),
            vault_a: vault_a.pubkey(),
            vault_b: vault_b.pubkey(),
            token_program: spl_token::id(),
        }
        .to_account_metas(None),
        data: instruction::Swap {
            amount_in: 100,
        }
        .data(),
    };

    let message = Message::new(
        &[swap_ix],
        Some(&authority.pubkey()),
    );

    let tx = Transaction::new(
        &[&authority],
        message,
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    // --------------------------------------------------
    // Verify swap
    // --------------------------------------------------

    // User gave 100 A
    assert_eq!(token_balance(&svm, &user_token_a.pubkey()), 400);

    // Vault received 100 A
    assert_eq!(token_balance(&svm, &vault_a.pubkey()), 600);

    // User received 84 B
    assert_eq!(token_balance(&svm, &user_token_b.pubkey()), 584);

    // Vault sent 84 B
    assert_eq!(token_balance(&svm, &vault_b.pubkey()), 416);
}