#![cfg(test)]

use borsh::{to_vec, BorshDeserialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    instruction::InstructionError,
    pubkey::Pubkey,
    system_instruction,
    system_program,
};
use solana_program_test::{processor, BanksClientError, ProgramTest, ProgramTestContext};
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};

use gulliant_v1::{
    error::GulliantError,
    instruction::GulliantInstruction,
    processor::Processor,
    state::{ExportAuthorizationState, MigratedStateLink, UserLogState},
};

const PROGRAM_ID: Pubkey = gulliant_v1::ID;

async fn setup() -> (ProgramTestContext, Keypair, Keypair) {
    let program_test =
        ProgramTest::new("gulliant_v1", PROGRAM_ID, processor!(Processor::process));
    let context = program_test.start_with_context().await;
    let protocol_keypair = Keypair::new();
    let user_keypair = Keypair::new();
    (context, protocol_keypair, user_keypair)
}

fn get_custom_error_code(err: BanksClientError) -> Option<u32> {
    match err {
        BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            instruction_err,
        )) => match instruction_err {
            InstructionError::Custom(code) => Some(code),
            _ => None,
        },
        _ => None,
    }
}

async fn fund_account(
    context: &mut ProgramTestContext,
    recipient: &Pubkey,
    lamports: u64,
) {
    let tx = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &context.payer.pubkey(),
            recipient,
            lamports,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
}

#[tokio::test]
async fn test_happy_path() {
    let (mut context, protocol_keypair, user_keypair) = setup().await;
    let wallet = user_keypair.pubkey();
    let protocol_id = protocol_keypair.pubkey();

    fund_account(&mut context, &wallet, 10_000_000).await;
    fund_account(&mut context, &protocol_id, 10_000_000).await;

    let (user_log_pda, _) = Pubkey::find_program_address(
        &[b"user_log", protocol_id.as_ref(), wallet.as_ref()],
        &PROGRAM_ID,
    );

    let init_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(wallet, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::InitializeUserLog { wallet, protocol_id }).unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let event_index = 0u64;
    let (event_pda, _) = Pubkey::find_program_address(
        &[
            b"activity_event",
            protocol_id.as_ref(),
            wallet.as_ref(),
            &event_index.to_le_bytes(),
        ],
        &PROGRAM_ID,
    );

    let append_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(event_pda, false),
            AccountMeta::new(protocol_id, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::AppendActivityEvent {
            wallet,
            protocol_id,
            event_type: 1,
            magnitude: 100,
            timestamp: 1000,
        })
        .unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[append_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &protocol_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let account = context
        .banks_client
        .get_account(user_log_pda)
        .await
        .unwrap()
        .unwrap();
    let mut user_log_slice: &[u8] = &account.data;
    let user_log = UserLogState::deserialize(&mut user_log_slice).unwrap();
    assert_eq!(user_log.event_count, 1);
    assert!(!user_log.is_migrated);

    let new_wallet = Keypair::new().pubkey();
    let (auth_pda, _) = Pubkey::find_program_address(
        &[
            b"export_auth",
            wallet.as_ref(),
            new_wallet.as_ref(),
            protocol_id.as_ref(),
        ],
        &PROGRAM_ID,
    );

    let auth_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(auth_pda, false),
            AccountMeta::new(protocol_id, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::AuthorizeExport {
            old_wallet: wallet,
            new_wallet,
            protocol_id,
            authorized_until: 2000,
        })
        .unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[auth_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &protocol_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let (new_user_log_pda, _) = Pubkey::find_program_address(
        &[b"user_log", protocol_id.as_ref(), new_wallet.as_ref()],
        &PROGRAM_ID,
    );
    let (link_pda, _) = Pubkey::find_program_address(
        &[
            b"migrated_link",
            wallet.as_ref(),
            new_wallet.as_ref(),
            protocol_id.as_ref(),
        ],
        &PROGRAM_ID,
    );

    let migrate_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(new_user_log_pda, false),
            AccountMeta::new(auth_pda, false),
            AccountMeta::new(link_pda, false),
            AccountMeta::new(wallet, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::MigrateState {
            old_wallet: wallet,
            new_wallet,
            protocol_id,
            current_timestamp: 1500,
        })
        .unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[migrate_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let old_account = context
        .banks_client
        .get_account(user_log_pda)
        .await
        .unwrap()
        .unwrap();
    let mut old_slice: &[u8] = &old_account.data;
    let old_log = UserLogState::deserialize(&mut old_slice).unwrap();
    assert!(old_log.is_migrated);
    assert_eq!(old_log.migrated_to, Some(new_wallet));

    let new_account = context
        .banks_client
        .get_account(new_user_log_pda)
        .await
        .unwrap()
        .unwrap();
    let mut new_slice: &[u8] = &new_account.data;
    let new_log = UserLogState::deserialize(&mut new_slice).unwrap();
    assert_eq!(new_log.event_count, 1);
    assert_eq!(new_log.last_hash, old_log.last_hash);

    let auth_account = context
        .banks_client
        .get_account(auth_pda)
        .await
        .unwrap()
        .unwrap();
    let mut auth_slice: &[u8] = &auth_account.data;
    let auth_state = ExportAuthorizationState::deserialize(&mut auth_slice).unwrap();
    assert!(auth_state.used);

    let link_account = context
        .banks_client
        .get_account(link_pda)
        .await
        .unwrap()
        .unwrap();
    let mut link_slice: &[u8] = &link_account.data;
    let link = MigratedStateLink::deserialize(&mut link_slice).unwrap();
    assert_eq!(link.old_wallet, wallet);
    assert_eq!(link.new_wallet, new_wallet);
}

#[tokio::test]
async fn test_missing_protocol_signature() {
    let (mut context, protocol_keypair, user_keypair) = setup().await;
    let wallet = user_keypair.pubkey();
    let protocol_id = protocol_keypair.pubkey();

    fund_account(&mut context, &wallet, 10_000_000).await;
    fund_account(&mut context, &protocol_id, 10_000_000).await;

    let (user_log_pda, _) = Pubkey::find_program_address(
        &[b"user_log", protocol_id.as_ref(), wallet.as_ref()],
        &PROGRAM_ID,
    );

    let init_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(wallet, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::InitializeUserLog { wallet, protocol_id }).unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let event_index = 0u64;
    let (event_pda, _) = Pubkey::find_program_address(
        &[
            b"activity_event",
            protocol_id.as_ref(),
            wallet.as_ref(),
            &event_index.to_le_bytes(),
        ],
        &PROGRAM_ID,
    );

    let append_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(event_pda, false),
            AccountMeta::new(protocol_id, false),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::AppendActivityEvent {
            wallet,
            protocol_id,
            event_type: 1,
            magnitude: 100,
            timestamp: 1000,
        })
        .unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[append_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    let err = context.banks_client.process_transaction(tx).await.unwrap_err();
    let code = get_custom_error_code(err).unwrap();
    assert_eq!(code, GulliantError::MissingProtocolSignature as u32);
}

#[tokio::test]
async fn test_snapshot_mismatch() {
    let (mut context, protocol_keypair, user_keypair) = setup().await;
    let wallet = user_keypair.pubkey();
    let protocol_id = protocol_keypair.pubkey();

    fund_account(&mut context, &wallet, 10_000_000).await;
    fund_account(&mut context, &protocol_id, 10_000_000).await;

    let (user_log_pda, _) = Pubkey::find_program_address(
        &[b"user_log", protocol_id.as_ref(), wallet.as_ref()],
        &PROGRAM_ID,
    );

    let init_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(wallet, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::InitializeUserLog { wallet, protocol_id }).unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let event_index = 0u64;
    let (event_pda, _) = Pubkey::find_program_address(
        &[
            b"activity_event",
            protocol_id.as_ref(),
            wallet.as_ref(),
            &event_index.to_le_bytes(),
        ],
        &PROGRAM_ID,
    );

    let append_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(event_pda, false),
            AccountMeta::new(protocol_id, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::AppendActivityEvent {
            wallet,
            protocol_id,
            event_type: 1,
            magnitude: 100,
            timestamp: 1000,
        })
        .unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[append_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &protocol_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let new_wallet = Keypair::new().pubkey();
    let (auth_pda, _) = Pubkey::find_program_address(
        &[
            b"export_auth",
            wallet.as_ref(),
            new_wallet.as_ref(),
            protocol_id.as_ref(),
        ],
        &PROGRAM_ID,
    );

    let auth_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(auth_pda, false),
            AccountMeta::new(protocol_id, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::AuthorizeExport {
            old_wallet: wallet,
            new_wallet,
            protocol_id,
            authorized_until: 2000,
        })
        .unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[auth_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &protocol_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let event_index2 = 1u64;
    let (event_pda2, _) = Pubkey::find_program_address(
        &[
            b"activity_event",
            protocol_id.as_ref(),
            wallet.as_ref(),
            &event_index2.to_le_bytes(),
        ],
        &PROGRAM_ID,
    );

    let append_ix2 = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(event_pda2, false),
            AccountMeta::new(protocol_id, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::AppendActivityEvent {
            wallet,
            protocol_id,
            event_type: 2,
            magnitude: 50,
            timestamp: 1100,
        })
        .unwrap(),
    };

    let tx2 = Transaction::new_signed_with_payer(
        &[append_ix2],
        Some(&context.payer.pubkey()),
        &[&context.payer, &protocol_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx2).await.unwrap();

    let (new_user_log_pda, _) = Pubkey::find_program_address(
        &[b"user_log", protocol_id.as_ref(), new_wallet.as_ref()],
        &PROGRAM_ID,
    );
    let (link_pda, _) = Pubkey::find_program_address(
        &[
            b"migrated_link",
            wallet.as_ref(),
            new_wallet.as_ref(),
            protocol_id.as_ref(),
        ],
        &PROGRAM_ID,
    );

    let migrate_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(new_user_log_pda, false),
            AccountMeta::new(auth_pda, false),
            AccountMeta::new(link_pda, false),
            AccountMeta::new(wallet, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::MigrateState {
            old_wallet: wallet,
            new_wallet,
            protocol_id,
            current_timestamp: 1500,
        })
        .unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[migrate_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_keypair],
        context.last_blockhash,
    );

    let err = context.banks_client.process_transaction(tx).await.unwrap_err();
    let code = get_custom_error_code(err).unwrap();
    assert_eq!(code, GulliantError::SnapshotHashMismatch as u32);
}

#[tokio::test]
async fn test_replay_attempt() {
    let (mut context, protocol_keypair, user_keypair) = setup().await;
    let wallet = user_keypair.pubkey();
    let protocol_id = protocol_keypair.pubkey();

    fund_account(&mut context, &wallet, 10_000_000).await;
    fund_account(&mut context, &protocol_id, 10_000_000).await;

    let (user_log_pda, _) = Pubkey::find_program_address(
        &[b"user_log", protocol_id.as_ref(), wallet.as_ref()],
        &PROGRAM_ID,
    );

    let init_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(wallet, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::InitializeUserLog { wallet, protocol_id }).unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let (event_pda, _) = Pubkey::find_program_address(
        &[
            b"activity_event",
            protocol_id.as_ref(),
            wallet.as_ref(),
            &0u64.to_le_bytes(),
        ],
        &PROGRAM_ID,
    );

    let append_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(event_pda, false),
            AccountMeta::new(protocol_id, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::AppendActivityEvent {
            wallet,
            protocol_id,
            event_type: 1,
            magnitude: 100,
            timestamp: 1000,
        })
        .unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[append_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &protocol_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let new_wallet = Keypair::new().pubkey();
    let (auth_pda, _) = Pubkey::find_program_address(
        &[
            b"export_auth",
            wallet.as_ref(),
            new_wallet.as_ref(),
            protocol_id.as_ref(),
        ],
        &PROGRAM_ID,
    );

    let auth_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(auth_pda, false),
            AccountMeta::new(protocol_id, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::AuthorizeExport {
            old_wallet: wallet,
            new_wallet,
            protocol_id,
            authorized_until: 2000,
        })
        .unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[auth_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &protocol_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let (new_user_log_pda, _) = Pubkey::find_program_address(
        &[b"user_log", protocol_id.as_ref(), new_wallet.as_ref()],
        &PROGRAM_ID,
    );
    let (link_pda, _) = Pubkey::find_program_address(
        &[
            b"migrated_link",
            wallet.as_ref(),
            new_wallet.as_ref(),
            protocol_id.as_ref(),
        ],
        &PROGRAM_ID,
    );

    let migrate_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user_log_pda, false),
            AccountMeta::new(new_user_log_pda, false),
            AccountMeta::new(auth_pda, false),
            AccountMeta::new(link_pda, false),
            AccountMeta::new(wallet, true),
            AccountMeta::new(system_program::ID, false),
        ],
        data: to_vec(&GulliantInstruction::MigrateState {
            old_wallet: wallet,
            new_wallet,
            protocol_id,
            current_timestamp: 1500,
        })
        .unwrap(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[migrate_ix.clone()],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();

    let old_log_account = context
        .banks_client
        .get_account(user_log_pda)
        .await
        .unwrap()
        .unwrap();
    let mut old_log_slice: &[u8] = &old_log_account.data;
    let old_log = UserLogState::deserialize(&mut old_log_slice).unwrap();

    assert!(old_log.is_migrated);
    assert_eq!(old_log.migrated_to, Some(new_wallet));

    let auth_account = context
        .banks_client
        .get_account(auth_pda)
        .await
        .unwrap()
        .unwrap();
    let mut auth_slice: &[u8] = &auth_account.data;
    let auth_state = ExportAuthorizationState::deserialize(&mut auth_slice).unwrap();

    assert!(auth_state.used);

    let tx = Transaction::new_signed_with_payer(
        &[migrate_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_keypair],
        context.last_blockhash,
    );

    let err = context.banks_client.process_transaction(tx).await.unwrap_err();
    let code = get_custom_error_code(err).unwrap();

    assert!(
        code == GulliantError::ExportAuthorizationAlreadyUsed as u32
            || code == GulliantError::WalletAlreadyMigrated as u32,
        "unexpected replay error code: {}",
        code
    );
}
