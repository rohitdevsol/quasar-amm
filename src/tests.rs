extern crate std;

use quasar_svm::{ Account, ExecutionResult, Instruction, Pubkey, QuasarSvm, token::Mint };
use solana_address::Address;
use solana_keypair::{ Keypair, Signer };
use solana_program_option::COption;
use solana_program_pack::Pack;
use spl_token::state::Account as SplTokenAccount;

use quasar_amm_client::{
    InitializeInstruction,
    DepositInstruction,
    WithdrawInstruction,
    SwapInstruction,
};

// ---------------------------------------------------------------------------
// Shared test state — covers accounts needed by all instructions
// ---------------------------------------------------------------------------

struct State {
    // keypairs (signers)
    maker: Keypair,
    user: Keypair,

    // mints
    mint_x: Pubkey,
    mint_y: Pubkey,

    // PDAs
    config: Pubkey,
    mint_lp: Pubkey,

    // vaults (ATAs owned by config PDA)
    vault_x: Pubkey,
    vault_y: Pubkey,

    // user ATAs
    user_ata_x: Pubkey,
    user_ata_y: Pubkey,
    user_ata_lp: Pubkey,

    // params
    seed: u64,
    fee: u16,
}

// ---------------------------------------------------------------------------
// Mint spec for test mints
// ---------------------------------------------------------------------------

const MINT_SPEC: Mint = Mint {
    is_initialized: true,
    freeze_authority: COption::None,
    decimals: 6,
    mint_authority: COption::None,
    supply: 100_000_000_000,
};

const FEE: u16 = 30; // 0.3% fee in basis points
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_SEED: AtomicU64 = AtomicU64::new(42);


// ---------------------------------------------------------------------------
// Helper — read an SPL token account from SVM state
// ---------------------------------------------------------------------------

fn read_token_account(svm: &QuasarSvm, address: &Pubkey) -> SplTokenAccount {
    let acct = svm
        .get_account(address)
        .unwrap_or_else(|| panic!("token account {} not found", address));
    SplTokenAccount::unpack(&acct.data).unwrap()
}

fn read_mint(svm: &QuasarSvm, address: &Pubkey) -> spl_token::state::Mint {
    let acct = svm
        .get_account(address)
        .unwrap_or_else(|| panic!("mint account {} not found", address));
    spl_token::state::Mint::unpack(&acct.data).unwrap()
}

// ---------------------------------------------------------------------------
// Setup — builds the SVM + pre-populated accounts
// ---------------------------------------------------------------------------

fn setup() -> (QuasarSvm, State) {
    let program_id = Pubkey::from(crate::ID);
    let elf = include_bytes!("../target/deploy/quasar_amm.so");
    let mut svm = QuasarSvm::new().with_program(&program_id, elf);

    let maker = Keypair::new();
    let user = Keypair::new();
    let mint_x = Pubkey::new_unique();
    let mint_y = Pubkey::new_unique();
    let seed = NEXT_SEED.fetch_add(1, Ordering::SeqCst);

    let mut out = String::new();
    out.push_str("\n==== Generating new independent state with random accounts ====\n");
    out.push_str(&format!("==== Seed ==== {}\n", seed));
    out.push_str(&format!("==== Maker ==== {}\n", maker.pubkey()));
    out.push_str(&format!("==== User ==== {}\n", user.pubkey()));
    out.push_str(&format!("==== Mint X ==== {}\n", mint_x));
    out.push_str(&format!("==== Mint Y ==== {}\n\n", mint_y));

    // derive config PDA: seeds = [b"config", seed.to_le_bytes()]
    let (config, _) = Pubkey::find_program_address(&[b"config", &seed.to_le_bytes()], &crate::ID);

    out.push_str(&format!("==== Config ==== {}\n", config));
    // derive mint_lp PDA: seeds = [b"lp", config]
    let (mint_lp, _) = Pubkey::find_program_address(&[b"lp", config.as_ref()], &crate::ID);

    out.push_str(&format!("==== Mint LP ==== {}\n", mint_lp));
    // derive vault ATAs — owned by config PDA
    let (vault_x, _) = Pubkey::find_program_address(
        &[config.as_ref(), quasar_svm::SPL_TOKEN_PROGRAM_ID.as_ref(), mint_x.as_ref()],
        &quasar_svm::SPL_ASSOCIATED_TOKEN_PROGRAM_ID
    );

    out.push_str(&format!("==== Vault X ==== {}\n", vault_x));

    let (vault_y, _) = Pubkey::find_program_address(
        &[config.as_ref(), quasar_svm::SPL_TOKEN_PROGRAM_ID.as_ref(), mint_y.as_ref()],
        &quasar_svm::SPL_ASSOCIATED_TOKEN_PROGRAM_ID
    );
    out.push_str(&format!("==== Vault Y ==== {}\n\n", vault_y));
    // derive user ATAs
    let (user_ata_x, _) = Pubkey::find_program_address(
        &[user.pubkey().as_ref(), quasar_svm::SPL_TOKEN_PROGRAM_ID.as_ref(), mint_x.as_ref()],
        &quasar_svm::SPL_ASSOCIATED_TOKEN_PROGRAM_ID
    );

    out.push_str(&format!("==== User ATA X ==== {}\n", user_ata_x));

    let (user_ata_y, _) = Pubkey::find_program_address(
        &[user.pubkey().as_ref(), quasar_svm::SPL_TOKEN_PROGRAM_ID.as_ref(), mint_y.as_ref()],
        &quasar_svm::SPL_ASSOCIATED_TOKEN_PROGRAM_ID
    );
    out.push_str(&format!("==== User ATA Y ==== {}\n", user_ata_y));

    let (user_ata_lp, _) = Pubkey::find_program_address(
        &[user.pubkey().as_ref(), quasar_svm::SPL_TOKEN_PROGRAM_ID.as_ref(), mint_lp.as_ref()],
        &quasar_svm::SPL_ASSOCIATED_TOKEN_PROGRAM_ID
    );

    out.push_str(&format!("==== User ATA LP ==== {}\n\n", user_ata_lp));
    print!("{}", out);

    // --- fund maker (pool creator / admin) ---
    svm.set_account(Account {
        address: maker.pubkey(),
        lamports: 20_000_000_000,
        data: vec![],
        owner: quasar_svm::system_program::ID,
        executable: false,
    });

    // --- fund user (liquidity provider / swapper) ---
    svm.set_account(Account {
        address: user.pubkey(),
        lamports: 20_000_000_000,
        data: vec![],
        owner: quasar_svm::system_program::ID,
        executable: false,
    });

    // --- create mints ---
    svm.set_account(quasar_svm::token::create_keyed_mint_account(&mint_x, &MINT_SPEC));
    svm.set_account(quasar_svm::token::create_keyed_mint_account(&mint_y, &MINT_SPEC));

    // --- user ATAs with token balances for depositing ---
    svm.set_account(
        quasar_svm::token::create_keyed_associated_token_account(
            &user.pubkey(),
            &mint_x,
            50_000_000_000
        )
    );

    svm.set_account(
        quasar_svm::token::create_keyed_associated_token_account(
            &user.pubkey(),
            &mint_y,
            50_000_000_000
        )
    );

    // --- Register PDAs and ATAs as empty system accounts so SVM tracks their creations ---
    svm.set_account(quasar_svm::token::create_keyed_system_account(&config, 0));
    svm.set_account(quasar_svm::token::create_keyed_system_account(&mint_lp, 0));
    svm.set_account(quasar_svm::token::create_keyed_system_account(&vault_x, 0));
    svm.set_account(quasar_svm::token::create_keyed_system_account(&vault_y, 0));
    svm.set_account(quasar_svm::token::create_keyed_system_account(&user_ata_lp, 0));

    let state = State {
        maker,
        user,
        mint_x,
        mint_y,
        config,
        mint_lp,
        vault_x,
        vault_y,
        user_ata_x,
        user_ata_y,
        user_ata_lp,
        seed,
        fee: FEE,
    };

    (svm, state)
}

// ---------------------------------------------------------------------------
// Instruction runners — reusable helpers for composing tests
// ---------------------------------------------------------------------------

fn run_initialize(svm: &mut QuasarSvm, state: &State) -> ExecutionResult {
    let ix: Instruction = (InitializeInstruction {
        maker: Address::from(state.maker.pubkey().to_bytes()),
        mint_x: Address::from(state.mint_x.to_bytes()),
        mint_y: Address::from(state.mint_y.to_bytes()),
        vault_x: Address::from(state.vault_x.to_bytes()),
        vault_y: Address::from(state.vault_y.to_bytes()),
        config: Address::from(state.config.to_bytes()),
        mint_lp: Address::from(state.mint_lp.to_bytes()),
        token_program: Address::from(quasar_svm::SPL_TOKEN_PROGRAM_ID.to_bytes()),
        system_program: Address::from(quasar_svm::system_program::ID.to_bytes()),
        associated_token_program: Address::from(
            quasar_svm::SPL_ASSOCIATED_TOKEN_PROGRAM_ID.to_bytes()
        ),
        fee: state.fee,
        seed: state.seed,
    }).into();

    svm.process_instruction(&ix, &[])
}

fn run_deposit(
    svm: &mut QuasarSvm,
    state: &State,
    amount: u64,
    max_x: u64,
    max_y: u64
) -> ExecutionResult {
    let ix: Instruction = (DepositInstruction {
        user: Address::from(state.user.pubkey().to_bytes()),
        mint_x: Address::from(state.mint_x.to_bytes()),
        mint_y: Address::from(state.mint_y.to_bytes()),
        config: Address::from(state.config.to_bytes()),
        mint_lp: Address::from(state.mint_lp.to_bytes()),
        vault_x: Address::from(state.vault_x.to_bytes()),
        vault_y: Address::from(state.vault_y.to_bytes()),
        user_ata_x: Address::from(state.user_ata_x.to_bytes()),
        user_ata_y: Address::from(state.user_ata_y.to_bytes()),
        user_ata_lp: Address::from(state.user_ata_lp.to_bytes()),
        token_program: Address::from(quasar_svm::SPL_TOKEN_PROGRAM_ID.to_bytes()),
        system_program: Address::from(quasar_svm::system_program::ID.to_bytes()),
        associated_token_program: Address::from(
            quasar_svm::SPL_ASSOCIATED_TOKEN_PROGRAM_ID.to_bytes()
        ),
        amount,
        max_x,
        max_y,
    }).into();

    svm.process_instruction(&ix, &[])
}

fn run_withdraw(
    svm: &mut QuasarSvm,
    state: &State,
    amount: u64,
    min_x: u64,
    min_y: u64
) -> ExecutionResult {
    let ix: Instruction = (WithdrawInstruction {
        user: Address::from(state.user.pubkey().to_bytes()),
        mint_x: Address::from(state.mint_x.to_bytes()),
        mint_y: Address::from(state.mint_y.to_bytes()),
        config: Address::from(state.config.to_bytes()),
        mint_lp: Address::from(state.mint_lp.to_bytes()),
        vault_x: Address::from(state.vault_x.to_bytes()),
        vault_y: Address::from(state.vault_y.to_bytes()),
        user_ata_x: Address::from(state.user_ata_x.to_bytes()),
        user_ata_y: Address::from(state.user_ata_y.to_bytes()),
        user_ata_lp: Address::from(state.user_ata_lp.to_bytes()),
        token_program: Address::from(quasar_svm::SPL_TOKEN_PROGRAM_ID.to_bytes()),
        system_program: Address::from(quasar_svm::system_program::ID.to_bytes()),
        associated_token_program: Address::from(
            quasar_svm::SPL_ASSOCIATED_TOKEN_PROGRAM_ID.to_bytes()
        ),
        amount,
        min_x,
        min_y,
    }).into();

    svm.process_instruction(&ix, &[])
}

fn run_swap(
    svm: &mut QuasarSvm,
    state: &State,
    is_x: bool,
    amount_in: u64,
    min_amount_out: u64
) -> ExecutionResult {
    let ix: Instruction = (SwapInstruction {
        user: Address::from(state.user.pubkey().to_bytes()),
        mint_x: Address::from(state.mint_x.to_bytes()),
        mint_y: Address::from(state.mint_y.to_bytes()),
        config: Address::from(state.config.to_bytes()),
        mint_lp: Address::from(state.mint_lp.to_bytes()),
        vault_x: Address::from(state.vault_x.to_bytes()),
        vault_y: Address::from(state.vault_y.to_bytes()),
        user_ata_x: Address::from(state.user_ata_x.to_bytes()),
        user_ata_y: Address::from(state.user_ata_y.to_bytes()),
        token_program: Address::from(quasar_svm::SPL_TOKEN_PROGRAM_ID.to_bytes()),
        system_program: Address::from(quasar_svm::system_program::ID.to_bytes()),
        associated_token_program: Address::from(
            quasar_svm::SPL_ASSOCIATED_TOKEN_PROGRAM_ID.to_bytes()
        ),
        is_x,
        amount_in,
        min_amount_out,
    }).into();

    svm.process_instruction(&ix, &[])
}



// ===========================================================================
// Tests — Initialize
// ===========================================================================

#[test]
fn test_initialize() {
    let (mut svm, state) = setup();
    let result = run_initialize(&mut svm, &state);
    result.assert_success();

    // vault_x should exist and be empty, owned by config PDA
    let vault_x = read_token_account(&svm, &state.vault_x);
    assert_eq!(vault_x.amount, 0);
    assert_eq!(vault_x.mint, state.mint_x);
    assert_eq!(vault_x.owner, state.config);

    // vault_y should exist and be empty, owned by config PDA
    let vault_y = read_token_account(&svm, &state.vault_y);
    assert_eq!(vault_y.amount, 0);
    assert_eq!(vault_y.mint, state.mint_y);
    assert_eq!(vault_y.owner, state.config);

    // mint_lp should be initialized with 0 supply and 6 decimals
    let mint_lp = read_mint(&svm, &state.mint_lp);
    assert_eq!(mint_lp.supply, 0);
    assert_eq!(mint_lp.decimals, 6);
    assert!(mint_lp.is_initialized);
}

#[test]
fn test_initialize_double_init_should_fail() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();

    // second init with same seed should fail (config PDA already exists)
    let result = run_initialize(&mut svm, &state);
    assert!(result.is_err(), "double init should fail");
}

// ===========================================================================
// Tests — Deposit
// ===========================================================================

#[test]
fn test_first_deposit() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();

    // first deposit: sets initial pool ratio
    let result = run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000);
    result.assert_success();

    // vault_x should have received deposited tokens
    let vault_x = read_token_account(&svm, &state.vault_x);
    assert_eq!(vault_x.amount, 10_000_000_000);

    // vault_y should have received deposited tokens
    let vault_y = read_token_account(&svm, &state.vault_y);
    assert_eq!(vault_y.amount, 10_000_000_000);

    // user should have received LP tokens
    let user_lp = read_token_account(&svm, &state.user_ata_lp);
    assert_eq!(user_lp.amount, 1_000_000);

    // user_ata_x balance should have decreased
    let user_x = read_token_account(&svm, &state.user_ata_x);
    assert_eq!(user_x.amount, 40_000_000_000);

    // user_ata_y balance should have decreased
    let user_y = read_token_account(&svm, &state.user_ata_y);
    assert_eq!(user_y.amount, 40_000_000_000);
}

#[test]
fn test_deposit_zero_lp_should_fail() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();

    // requesting 0 LP tokens should fail
    let result = run_deposit(&mut svm, &state, 0, 10_000_000_000, 10_000_000_000);
    assert!(result.is_err(), "zero LP deposit should fail");
}

#[test]
fn test_deposit_before_initialize_should_fail() {
    let (mut svm, state) = setup();

    // deposit without initializing the pool first should fail
    let result = run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000);
    assert!(result.is_err(), "deposit before init should fail");
}

// ===========================================================================
// Tests — Withdraw
// ===========================================================================

#[test]
fn test_withdraw_all_liquidity() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();
    run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000).assert_success();

    // withdraw all LP tokens
    let result = run_withdraw(&mut svm, &state, 1_000_000, 0, 0);
    result.assert_success();

    // vaults should be empty after full withdrawal
    let vault_x = read_token_account(&svm, &state.vault_x);
    assert_eq!(vault_x.amount, 0);

    let vault_y = read_token_account(&svm, &state.vault_y);
    assert_eq!(vault_y.amount, 0);

    // user should get back all tokens
    let user_x = read_token_account(&svm, &state.user_ata_x);
    assert_eq!(user_x.amount, 50_000_000_000);

    let user_y = read_token_account(&svm, &state.user_ata_y);
    assert_eq!(user_y.amount, 50_000_000_000);

    // LP tokens should be burned
    let user_lp = read_token_account(&svm, &state.user_ata_lp);
    assert_eq!(user_lp.amount, 0);
}

#[test]
fn test_withdraw_partial_liquidity() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();
    run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000).assert_success();

    // withdraw half of LP tokens
    let result = run_withdraw(&mut svm, &state, 500_000, 0, 0);
    result.assert_success();

    // vaults should retain ~half the liquidity
    let vault_x = read_token_account(&svm, &state.vault_x);
    assert_eq!(vault_x.amount, 5_000_000_000);

    let vault_y = read_token_account(&svm, &state.vault_y);
    assert_eq!(vault_y.amount, 5_000_000_000);

    // user LP balance should be halved
    let user_lp = read_token_account(&svm, &state.user_ata_lp);
    assert_eq!(user_lp.amount, 500_000);
}

#[test]
fn test_withdraw_zero_lp_should_fail() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();
    run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000).assert_success();

    // withdrawing 0 LP tokens should fail
    let result = run_withdraw(&mut svm, &state, 0, 0, 0);
    assert!(result.is_err(), "zero LP withdraw should fail");
}

#[test]
fn test_withdraw_slippage_protection() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();
    run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000).assert_success();

    // set min_x higher than what we'd receive — should fail with slippage error
    let result = run_withdraw(&mut svm, &state, 500_000, 999_999_999_999, 0);
    assert!(result.is_err(), "withdraw slippage should fail");
}

// ===========================================================================
// Tests — Swap
// ===========================================================================

#[test]
fn test_swap_x_for_y() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();
    run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000).assert_success();

    // swap token X for token Y
    let result = run_swap(&mut svm, &state, true, 1_000_000_000, 0);
    result.assert_success();

    // vault_x should increase (received user's X)
    let vault_x = read_token_account(&svm, &state.vault_x);
    assert!(vault_x.amount > 10_000_000_000);

    // vault_y should decrease (sent Y to user)
    let vault_y = read_token_account(&svm, &state.vault_y);
    assert!(vault_y.amount < 10_000_000_000);

    // user_ata_x should decrease
    let user_x = read_token_account(&svm, &state.user_ata_x);
    assert!(user_x.amount < 40_000_000_000);

    // user_ata_y should increase
    let user_y = read_token_account(&svm, &state.user_ata_y);
    assert!(user_y.amount > 40_000_000_000);
}

#[test]
fn test_swap_y_for_x() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();
    run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000).assert_success();

    // swap token Y for token X
    let result = run_swap(&mut svm, &state, false, 1_000_000_000, 0);
    result.assert_success();

    // vault_y should increase (received user's Y)
    let vault_y = read_token_account(&svm, &state.vault_y);
    assert!(vault_y.amount > 10_000_000_000);

    // vault_x should decrease (sent X to user)
    let vault_x = read_token_account(&svm, &state.vault_x);
    assert!(vault_x.amount < 10_000_000_000);
}

#[test]
fn test_swap_zero_amount_should_fail() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();
    run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000).assert_success();

    // swapping 0 tokens should fail
    let result = run_swap(&mut svm, &state, true, 0, 0);
    assert!(result.is_err(), "zero swap should fail");
}

#[test]
fn test_swap_slippage_protection() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();
    run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000).assert_success();

    // set min_amount_out impossibly high — should fail with slippage error
    let result = run_swap(&mut svm, &state, true, 1_000_000_000, 999_999_999_999);
    assert!(result.is_err(), "swap slippage should fail");
}

#[test]
fn test_swap_constant_product_invariant() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();
    run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000).assert_success();

    // k = x * y before swap (minus fee effects)
    let k_before: u128 = 10_000_000_000u128 * 10_000_000_000u128;

    let result = run_swap(&mut svm, &state, true, 1_000_000_000, 0);
    result.assert_success();

    let vault_x = read_token_account(&svm, &state.vault_x);
    let vault_y = read_token_account(&svm, &state.vault_y);

    // k after swap should be >= k before (fees only increase k)
    let k_after: u128 = (vault_x.amount as u128) * (vault_y.amount as u128);
    assert!(
        k_after >= k_before,
        "constant product invariant violated: k_before={}, k_after={}",
        k_before,
        k_after
    );
}



// ===========================================================================
// Tests — Swap on empty pool / before deposit
// ===========================================================================

#[test]
fn test_swap_on_empty_pool_should_fail() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();

    // swap on a pool with no liquidity should fail
    let result = run_swap(&mut svm, &state, true, 1_000_000_000, 0);
    assert!(result.is_err(), "swap on empty pool should fail");
}

// ===========================================================================
// Tests — Deposit after swap (proportional deposit)
// ===========================================================================

#[test]
fn test_second_deposit_proportional() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();

    // first deposit establishes ratio
    run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000).assert_success();

    // swap to skew the ratio
    run_swap(&mut svm, &state, true, 1_000_000_000, 0).assert_success();

    // second deposit — must maintain the curve, so max_x/max_y should be generous
    let result = run_deposit(&mut svm, &state, 100_000, 20_000_000_000, 20_000_000_000);
    result.assert_success();

    // user should have received more LP tokens total
    let user_lp = read_token_account(&svm, &state.user_ata_lp);
    assert_eq!(user_lp.amount, 1_100_000);
}

// ===========================================================================
// Tests — Deposit slippage protection
// ===========================================================================

// #[test]
// fn test_deposit_slippage_exceeded() {
//     let (mut svm, state) = setup();
//     run_initialize(&mut svm, &state).assert_success();
//     run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000).assert_success();

//     // swap to skew ratio, then try deposit with tight max — should fail
//     run_swap(&mut svm, &state, true, 1_000_000_000, 0).assert_success();

//     // max_x and max_y set to 1 — way too low for proportional deposit
//     let result = run_deposit(&mut svm, &state, 100_000, 1, 1);
//     assert!(result.is_err(), "deposit slippage exceeded should fail");
// }

// ===========================================================================
// Tests — Full lifecycle (initialize → deposit → swap → withdraw)
// ===========================================================================

#[test]
fn test_full_lifecycle() {
    let (mut svm, state) = setup();

    // 1. initialize pool
    run_initialize(&mut svm, &state).assert_success();

    // 2. deposit liquidity
    run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000).assert_success();

    // 3. swap X→Y
    run_swap(&mut svm, &state, true, 500_000_000, 0).assert_success();

    // 4. swap Y→X
    run_swap(&mut svm, &state, false, 500_000_000, 0).assert_success();

    // 5. withdraw all liquidity
    let result = run_withdraw(&mut svm, &state, 1_000_000, 0, 0);
    result.assert_success();

    // vaults should be empty
    let vault_x = read_token_account(&svm, &state.vault_x);
    let vault_y = read_token_account(&svm, &state.vault_y);
    assert_eq!(vault_x.amount, 0);
    assert_eq!(vault_y.amount, 0);
}

#[test]
fn test_withdraw_more_than_lp_balance_should_fail() {
    let (mut svm, state) = setup();
    run_initialize(&mut svm, &state).assert_success();
    run_deposit(&mut svm, &state, 1_000_000, 10_000_000_000, 10_000_000_000).assert_success();

    // try to withdraw more LP tokens than user has
    let result = run_withdraw(&mut svm, &state, 2_000_000, 0, 0);
    assert!(result.is_err(), "withdraw more than balance should fail");
}
