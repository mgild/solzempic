use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction,
    transaction::Transaction,
};

const PROGRAM_ID: Pubkey = solana_sdk::pubkey!("SoLzeMPic1111111111111111111111111111111111");

fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../target/deploy/solzempic_test_program.so");
    svm.add_program(PROGRAM_ID, program_bytes);
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
}

/// Create a counter account (pre-allocated, owned by program)
fn create_counter_account(svm: &mut LiteSVM, payer: &Keypair) -> Keypair {
    let counter = Keypair::new();
    let space = std::mem::size_of::<solzempic_test_program::Counter>();
    let rent = svm.minimum_balance_for_rent_exemption(space);

    let create_ix = system_instruction::create_account(
        &payer.pubkey(),
        &counter.pubkey(),
        rent,
        space as u64,
        &PROGRAM_ID,
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&payer.pubkey()),
        &[payer, &counter],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    counter
}

fn build_init_counter_ix(payer: &Pubkey, counter: &Pubkey, initial_count: u64) -> Instruction {
    // Discriminator byte (0 = InitCounter) + params
    let mut data = vec![0u8]; // InitCounter discriminator
    data.extend_from_slice(&initial_count.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*payer, true),        // payer/signer
            AccountMeta::new(*counter, false),      // counter account
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // system program
        ],
        data,
    }
}

fn build_increment_ix(counter: &Pubkey, owner: &Pubkey, amount: u64) -> Instruction {
    let mut data = vec![1u8]; // Increment discriminator
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*counter, false), // counter account
            AccountMeta::new_readonly(*owner, true), // owner/signer
        ],
        data,
    }
}

fn build_transfer_sol_ix(from: &Pubkey, to: &Pubkey, amount: u64) -> Instruction {
    let mut data = vec![2u8]; // TransferSol discriminator
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*from, true),  // from/signer
            AccountMeta::new(*to, false),   // to
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // system program
        ],
        data,
    }
}

fn read_counter(svm: &LiteSVM, counter: &Pubkey) -> (Pubkey, u64) {
    let account = svm.get_account(counter).unwrap();
    let data = &account.data;
    // Skip 8-byte discriminator
    let owner_bytes: [u8; 32] = data[8..40].try_into().unwrap();
    let count = u64::from_le_bytes(data[40..48].try_into().unwrap());
    (Pubkey::from(owner_bytes), count)
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_init_counter() {
    let (mut svm, payer) = setup();
    let counter = create_counter_account(&mut svm, &payer);

    let ix = build_init_counter_ix(&payer.pubkey(), &counter.pubkey(), 42);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "InitCounter failed: {:?}", result.err());

    // Verify counter state
    let (owner, count) = read_counter(&svm, &counter.pubkey());
    assert_eq!(owner, payer.pubkey());
    assert_eq!(count, 42);
}

#[test]
fn test_init_counter_zero() {
    let (mut svm, payer) = setup();
    let counter = create_counter_account(&mut svm, &payer);

    let ix = build_init_counter_ix(&payer.pubkey(), &counter.pubkey(), 0);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (owner, count) = read_counter(&svm, &counter.pubkey());
    assert_eq!(owner, payer.pubkey());
    assert_eq!(count, 0);
}

#[test]
fn test_increment() {
    let (mut svm, payer) = setup();
    let counter = create_counter_account(&mut svm, &payer);

    // Init
    let ix = build_init_counter_ix(&payer.pubkey(), &counter.pubkey(), 10);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Increment by 5
    let ix = build_increment_ix(&counter.pubkey(), &payer.pubkey(), 5);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Increment failed: {:?}", result.err());

    let (_, count) = read_counter(&svm, &counter.pubkey());
    assert_eq!(count, 15);
}

#[test]
fn test_increment_multiple() {
    let (mut svm, payer) = setup();
    let counter = create_counter_account(&mut svm, &payer);

    // Init with 0
    let ix = build_init_counter_ix(&payer.pubkey(), &counter.pubkey(), 0);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Increment 3 times
    for i in 1..=3u64 {
        let ix = build_increment_ix(&counter.pubkey(), &payer.pubkey(), i);
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer],
            svm.latest_blockhash(),
        );
        svm.send_transaction(tx).unwrap();
    }

    let (_, count) = read_counter(&svm, &counter.pubkey());
    assert_eq!(count, 6); // 0 + 1 + 2 + 3
}

#[test]
fn test_increment_wrong_owner_fails() {
    let (mut svm, payer) = setup();
    let counter = create_counter_account(&mut svm, &payer);

    // Init with payer as owner
    let ix = build_init_counter_ix(&payer.pubkey(), &counter.pubkey(), 10);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Try to increment with different signer
    let imposter = Keypair::new();
    svm.airdrop(&imposter.pubkey(), 1_000_000_000).unwrap();

    let ix = build_increment_ix(&counter.pubkey(), &imposter.pubkey(), 1);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&imposter.pubkey()),
        &[&imposter],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Should fail with wrong owner");
}

#[test]
fn test_increment_not_signer_fails() {
    let (mut svm, payer) = setup();
    let counter = create_counter_account(&mut svm, &payer);

    // Init with payer as owner
    let ix = build_init_counter_ix(&payer.pubkey(), &counter.pubkey(), 10);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Use a separate fee payer so the owner's pubkey isn't auto-marked as signer
    let fee_payer = Keypair::new();
    svm.airdrop(&fee_payer.pubkey(), 1_000_000_000).unwrap();

    // Build increment with owner's pubkey but mark it as non-signer
    let mut ix = build_increment_ix(&counter.pubkey(), &payer.pubkey(), 1);
    ix.accounts[1].is_signer = false;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fee_payer.pubkey()),
        &[&fee_payer], // only fee_payer signs, not the owner
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Should fail without signer");
}

#[test]
fn test_transfer_sol() {
    let (mut svm, payer) = setup();
    let recipient = Keypair::new();

    let transfer_amount = 1_000_000u64; // 0.001 SOL
    let ix = build_transfer_sol_ix(&payer.pubkey(), &recipient.pubkey(), transfer_amount);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "TransferSol failed: {:?}", result.err());

    let recipient_balance = svm.get_balance(&recipient.pubkey()).unwrap();
    assert_eq!(recipient_balance, transfer_amount);
}

#[test]
fn test_transfer_sol_zero_fails() {
    let (mut svm, payer) = setup();
    let recipient = Keypair::new();

    let ix = build_transfer_sol_ix(&payer.pubkey(), &recipient.pubkey(), 0);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Zero transfer should fail (validation)");
}

#[test]
fn test_invalid_discriminator_fails() {
    let (mut svm, payer) = setup();

    // Use invalid discriminator byte (255)
    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![AccountMeta::new(payer.pubkey(), true)],
        data: vec![255],
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Invalid discriminator should fail");
}

#[test]
fn test_empty_data_fails() {
    let (mut svm, payer) = setup();

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![AccountMeta::new(payer.pubkey(), true)],
        data: vec![],
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Empty data should fail");
}

#[test]
fn test_double_init_fails() {
    let (mut svm, payer) = setup();
    let counter = create_counter_account(&mut svm, &payer);

    // Init once
    let ix = build_init_counter_ix(&payer.pubkey(), &counter.pubkey(), 1);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Init again should fail (already initialized)
    let ix = build_init_counter_ix(&payer.pubkey(), &counter.pubkey(), 2);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Double init should fail");
}

#[test]
fn test_load_uninitialized_fails() {
    let (mut svm, payer) = setup();
    let counter = create_counter_account(&mut svm, &payer);

    // Try to increment without initializing first
    let ix = build_increment_ix(&counter.pubkey(), &payer.pubkey(), 1);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Load uninitialized should fail (wrong discriminator)");
}

#[test]
fn test_compute_units_init_counter() {
    let (mut svm, payer) = setup();
    let counter = create_counter_account(&mut svm, &payer);

    let ix = build_init_counter_ix(&payer.pubkey(), &counter.pubkey(), 42);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx).unwrap();
    let cus = result.compute_units_consumed;
    println!("InitCounter CUs: {cus}");
    // Framework overhead should be minimal - well under 10k CUs for a simple init
    assert!(cus < 10_000, "InitCounter used {cus} CUs - too many!");
}

#[test]
fn test_compute_units_increment() {
    let (mut svm, payer) = setup();
    let counter = create_counter_account(&mut svm, &payer);

    // Init first
    let ix = build_init_counter_ix(&payer.pubkey(), &counter.pubkey(), 0);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Measure increment
    let ix = build_increment_ix(&counter.pubkey(), &payer.pubkey(), 1);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx).unwrap();
    let cus = result.compute_units_consumed;
    println!("Increment CUs: {cus}");
    // Simple increment should be very cheap
    assert!(cus < 5_000, "Increment used {cus} CUs - too many!");
}

#[test]
fn test_compute_units_transfer_sol() {
    let (mut svm, payer) = setup();
    let recipient = Keypair::new();

    let ix = build_transfer_sol_ix(&payer.pubkey(), &recipient.pubkey(), 1_000_000);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx).unwrap();
    let cus = result.compute_units_consumed;
    println!("TransferSol CUs: {cus}");
    // SOL transfer via CPI should be reasonable
    assert!(cus < 10_000, "TransferSol used {cus} CUs - too many!");
}
