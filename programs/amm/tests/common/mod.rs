#![allow(dead_code)]

//! Shared setup for the AMM integration tests.
//!
//! Every test starts from `TestEnv::new(fee_bps)`, which boots LiteSVM, creates
//! two mints, and runs `initialize` so the pool, vaults, LP mint and treasuries
//! all exist.

use anchor_lang::{
    prelude::Pubkey,
    solana_program::{instruction::Instruction, program_pack::Pack, system_program},
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::token::spl_token;
use litesvm::{types::TransactionResult, LiteSVM};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::str::FromStr;

use amm_q3_w3::{state::Pool, LP_MINT_SEED, POOL_SEED};

pub const PROGRAM_ID: &str = "2KRXzBtBcv9dgKDx2KGF84N8XCHdJMLMse97hzYFpNMF";

const MINT_LEN: usize = 82;
const TOKEN_ACCOUNT_LEN: usize = 165;
const DECIMALS: u8 = 6;

/// 0.30%, the classic Uniswap V2 fee.
pub const DEFAULT_FEE_BPS: u16 = 30;

pub struct TestEnv {
    pub svm: LiteSVM,
    pub program_id: Pubkey,
    pub authority: Keypair,

    pub mint_a: Pubkey,
    pub mint_b: Pubkey,

    pub pool: Pubkey,
    pub pool_bump: u8,

    pub lp_mint: Pubkey,
    pub lp_bump: u8,

    pub vault_a: Pubkey,
    pub vault_b: Pubkey,

    pub treasury_a: Pubkey,
    pub treasury_b: Pubkey,
}

/// A trader / liquidity provider, with one token account per mint.
pub struct User {
    pub keypair: Keypair,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub lp: Pubkey,
}

impl User {
    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }
}

impl TestEnv {
    /// Boot the chain, create both mints, and run `initialize`.
    pub fn new(fee_bps: u16) -> Self {
        Self::try_new(fee_bps).expect("initialize failed")
    }

    /// Same as [`TestEnv::new`], but hands back the `initialize` failure instead
    /// of panicking - used by the tests that expect it to be rejected.
    pub fn try_new(fee_bps: u16) -> Result<Self, litesvm::types::FailedTransactionMetadata> {
        let mut svm = LiteSVM::new();

        let program = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/deploy/amm_q3_w3.so"
        ))
        .expect("run `anchor build` first");

        let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
        svm.add_program(program_id, &program).unwrap();

        let authority = Keypair::new();
        svm.airdrop(&authority.pubkey(), 100_000_000_000).unwrap();

        let (mint_a, mint_b) = create_two_mints(&mut svm, &authority);

        let (pool, pool_bump) = Pubkey::find_program_address(
            &[POOL_SEED, mint_a.as_ref(), mint_b.as_ref()],
            &program_id,
        );

        let (lp_mint, lp_bump) =
            Pubkey::find_program_address(&[LP_MINT_SEED, pool.as_ref()], &program_id);

        // Vaults and treasuries are plain token accounts created by `initialize`,
        // so they each need a fresh keypair that signs their own creation.
        let vault_a = Keypair::new();
        let vault_b = Keypair::new();
        let treasury_a = Keypair::new();
        let treasury_b = Keypair::new();

        let initialize_ix = Instruction {
            program_id,
            accounts: amm_q3_w3::accounts::Initialize {
                authority: authority.pubkey(),
                mint_a,
                mint_b,
                pool,
                vault_a: vault_a.pubkey(),
                vault_b: vault_b.pubkey(),
                lp_mint,
                treasury_a: treasury_a.pubkey(),
                treasury_b: treasury_b.pubkey(),
                system_program: system_program::ID,
                token_program: spl_token::ID,
            }
            .to_account_metas(None),
            data: amm_q3_w3::instruction::Initialize { fee_bps }.data(),
        };

        let tx = Transaction::new(
            &[&authority, &vault_a, &vault_b, &treasury_a, &treasury_b],
            Message::new(&[initialize_ix], Some(&authority.pubkey())),
            svm.latest_blockhash(),
        );

        svm.send_transaction(tx)?;

        Ok(Self {
            svm,
            program_id,
            authority,
            mint_a,
            mint_b,
            pool,
            pool_bump,
            lp_mint,
            lp_bump,
            vault_a: vault_a.pubkey(),
            vault_b: vault_b.pubkey(),
            treasury_a: treasury_a.pubkey(),
            treasury_b: treasury_b.pubkey(),
        })
    }

    /// A funded user holding `amount_a` of token A and `amount_b` of token B.
    pub fn create_user(&mut self, amount_a: u64, amount_b: u64) -> User {
        let keypair = Keypair::new();
        self.svm.airdrop(&keypair.pubkey(), 10_000_000_000).unwrap();

        let owner = keypair.pubkey();

        let token_a = self.create_token_account(self.mint_a, owner);
        let token_b = self.create_token_account(self.mint_b, owner);
        let lp = self.create_token_account(self.lp_mint, owner);

        if amount_a > 0 {
            self.mint_to(self.mint_a, token_a, amount_a);
        }

        if amount_b > 0 {
            self.mint_to(self.mint_b, token_b, amount_b);
        }

        User {
            keypair,
            token_a,
            token_b,
            lp,
        }
    }

    pub fn deposit(
        &mut self,
        user: &User,
        amount_a: u64,
        amount_b: u64,
        min_lp: u64,
    ) -> TransactionResult {
        let ix = Instruction {
            program_id: self.program_id,
            accounts: amm_q3_w3::accounts::Deposit {
                authority: user.pubkey(),
                pool: self.pool,
                lp_mint: self.lp_mint,
                user_token_a: user.token_a,
                user_token_b: user.token_b,
                user_lp: user.lp,
                vault_a: self.vault_a,
                vault_b: self.vault_b,
                token_program: spl_token::ID,
            }
            .to_account_metas(None),
            data: amm_q3_w3::instruction::Deposit {
                amount_a,
                amount_b,
                min_lp,
            }
            .data(),
        };

        self.send(&[&user.keypair], ix, &user.pubkey())
    }

    pub fn withdraw(
        &mut self,
        user: &User,
        lp_amount: u64,
        min_a: u64,
        min_b: u64,
    ) -> TransactionResult {
        let ix = Instruction {
            program_id: self.program_id,
            accounts: amm_q3_w3::accounts::Withdraw {
                authority: user.pubkey(),
                pool: self.pool,
                lp_mint: self.lp_mint,
                user_token_a: user.token_a,
                user_token_b: user.token_b,
                user_lp: user.lp,
                vault_a: self.vault_a,
                vault_b: self.vault_b,
                token_program: spl_token::ID,
            }
            .to_account_metas(None),
            data: amm_q3_w3::instruction::Withdraw {
                lp_amount,
                min_a,
                min_b,
            }
            .data(),
        };

        self.send(&[&user.keypair], ix, &user.pubkey())
    }

    pub fn swap(
        &mut self,
        user: &User,
        amount_in: u64,
        min_amount_out: u64,
        a_to_b: bool,
    ) -> TransactionResult {
        let ix = Instruction {
            program_id: self.program_id,
            accounts: amm_q3_w3::accounts::Swap {
                authority: user.pubkey(),
                pool: self.pool,
                user_token_a: user.token_a,
                user_token_b: user.token_b,
                vault_a: self.vault_a,
                vault_b: self.vault_b,
                treasury_a: self.treasury_a,
                treasury_b: self.treasury_b,
                token_program: spl_token::ID,
            }
            .to_account_metas(None),
            data: amm_q3_w3::instruction::Swap {
                amount_in,
                min_amount_out,
                a_to_b,
            }
            .data(),
        };

        self.send(&[&user.keypair], ix, &user.pubkey())
    }

    // --- reads -------------------------------------------------------------

    pub fn pool_state(&self) -> Pool {
        let account = self.svm.get_account(&self.pool).expect("pool missing");

        Pool::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    /// Token balance of any SPL token account.
    pub fn balance(&self, token_account: &Pubkey) -> u64 {
        self.token_account(token_account).amount
    }

    pub fn token_account(&self, key: &Pubkey) -> spl_token::state::Account {
        let account = self.svm.get_account(key).expect("token account missing");

        spl_token::state::Account::unpack(&account.data).unwrap()
    }

    pub fn lp_supply(&self) -> u64 {
        let account = self.svm.get_account(&self.lp_mint).expect("lp mint missing");

        spl_token::state::Mint::unpack(&account.data).unwrap().supply
    }

    /// `reserve_a * reserve_b` - the constant-product invariant.
    pub fn k(&self) -> u128 {
        self.balance(&self.vault_a) as u128 * self.balance(&self.vault_b) as u128
    }

    // --- plumbing ----------------------------------------------------------

    fn send(&mut self, signers: &[&Keypair], ix: Instruction, payer: &Pubkey) -> TransactionResult {
        // Fresh blockhash each time so two identical calls are distinct txs.
        self.svm.expire_blockhash();

        let tx = Transaction::new(
            signers,
            Message::new(&[ix], Some(payer)),
            self.svm.latest_blockhash(),
        );

        self.svm.send_transaction(tx)
    }

    pub fn create_token_account(&mut self, mint: Pubkey, owner: Pubkey) -> Pubkey {
        let account = Keypair::new();
        let rent = self
            .svm
            .minimum_balance_for_rent_exemption(TOKEN_ACCOUNT_LEN);

        let create = solana_system_interface::instruction::create_account(
            &self.authority.pubkey(),
            &account.pubkey(),
            rent,
            TOKEN_ACCOUNT_LEN as u64,
            &spl_token::ID,
        );

        let init = spl_token::instruction::initialize_account3(
            &spl_token::ID,
            &account.pubkey(),
            &mint,
            &owner,
        )
        .unwrap();

        self.svm.expire_blockhash();

        let tx = Transaction::new(
            &[&self.authority, &account],
            Message::new(&[create, init], Some(&self.authority.pubkey())),
            self.svm.latest_blockhash(),
        );

        self.svm.send_transaction(tx).unwrap();

        account.pubkey()
    }

    pub fn mint_to(&mut self, mint: Pubkey, destination: Pubkey, amount: u64) {
        let ix = spl_token::instruction::mint_to(
            &spl_token::ID,
            &mint,
            &destination,
            &self.authority.pubkey(),
            &[],
            amount,
        )
        .unwrap();

        self.svm.expire_blockhash();

        let tx = Transaction::new(
            &[&self.authority],
            Message::new(&[ix], Some(&self.authority.pubkey())),
            self.svm.latest_blockhash(),
        );

        self.svm.send_transaction(tx).unwrap();
    }
}

fn create_two_mints(svm: &mut LiteSVM, authority: &Keypair) -> (Pubkey, Pubkey) {
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();

    let rent = svm.minimum_balance_for_rent_exemption(MINT_LEN);

    let mut ixs = Vec::new();

    for mint in [&mint_a, &mint_b] {
        ixs.push(solana_system_interface::instruction::create_account(
            &authority.pubkey(),
            &mint.pubkey(),
            rent,
            MINT_LEN as u64,
            &spl_token::ID,
        ));

        ixs.push(
            spl_token::instruction::initialize_mint2(
                &spl_token::ID,
                &mint.pubkey(),
                &authority.pubkey(),
                None,
                DECIMALS,
            )
            .unwrap(),
        );
    }

    let tx = Transaction::new(
        &[authority, &mint_a, &mint_b],
        Message::new(&ixs, Some(&authority.pubkey())),
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    (mint_a.pubkey(), mint_b.pubkey())
}

/// Assert a transaction failed with a specific Anchor error.
pub fn assert_anchor_error(result: TransactionResult, expected: amm_q3_w3::error::AmmError) {
    let err = result.expect_err("expected the transaction to fail");

    let code = 6000 + expected as u32;
    let logs = err.meta.logs.join("\n");

    assert!(
        logs.contains(&format!("custom program error: {code:#x}"))
            || logs.contains(&format!("Error Number: {code}")),
        "expected error {expected:?} ({code}), got:\n{logs}"
    );
}
