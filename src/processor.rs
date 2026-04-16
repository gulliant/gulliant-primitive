use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction, system_program,
};
use solana_program::sysvar::Sysvar;

use crate::{
    error::GulliantError,
    instruction::GulliantInstruction,
    state::{
        ActivityEvent, ActivityEventAccount, ExportAuthorization, ExportAuthorizationState,
        MigratedStateLink, UserLogState,
    },
    utils::{compute_event_hash, verify_log_chain, verify_signer},
};

const USER_LOG_SEED: &[u8] = b"user_log";
const ACTIVITY_EVENT_SEED: &[u8] = b"activity_event";
const EXPORT_AUTH_SEED: &[u8] = b"export_auth";
const MIGRATED_LINK_SEED: &[u8] = b"migrated_link";

pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = GulliantInstruction::try_from_slice(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

        match instruction {
            GulliantInstruction::InitializeUserLog { wallet, protocol_id } => {
                Self::process_initialize_user_log(program_id, accounts, wallet, protocol_id)
            }
            GulliantInstruction::AppendActivityEvent {
                wallet,
                protocol_id,
                event_type,
                magnitude,
                timestamp,
            } => Self::process_append_activity_event(
                program_id, accounts, wallet, protocol_id, event_type, magnitude, timestamp,
            ),
            GulliantInstruction::AuthorizeExport {
                old_wallet,
                new_wallet,
                protocol_id,
                authorized_until,
            } => Self::process_authorize_export(
                program_id, accounts, old_wallet, new_wallet, protocol_id, authorized_until,
            ),
            GulliantInstruction::MigrateState {
                old_wallet,
                new_wallet,
                protocol_id,
                current_timestamp,
            } => Self::process_migrate_state(
                program_id, accounts, old_wallet, new_wallet, protocol_id, current_timestamp,
            ),
        }
    }

    fn process_initialize_user_log(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        wallet: Pubkey,
        protocol_id: Pubkey,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let user_log_account = next_account_info(accounts_iter)?;
        let user = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        if !verify_signer(user) {
            return Err(GulliantError::MissingUserSignature.into());
        }
        if user.key != &wallet {
            return Err(GulliantError::MissingUserSignature.into());
        }

        let (expected_pda, bump) = Pubkey::find_program_address(
            &[USER_LOG_SEED, protocol_id.as_ref(), wallet.as_ref()],
            program_id,
        );
        if expected_pda != *user_log_account.key {
            return Err(ProgramError::InvalidAccountData);
        }

        if user_log_account.owner == program_id {
            let data = user_log_account.try_borrow_data()?;
            let mut data_slice: &[u8] = &data;
            let state = UserLogState::deserialize(&mut data_slice)?;
            if state.is_initialized {
                return Err(ProgramError::AccountAlreadyInitialized);
            }
        } else if *user_log_account.owner == system_program::ID {
            if user_log_account.data_len() > 0 {
                return Err(ProgramError::AccountAlreadyInitialized);
            }

            let rent = solana_program::rent::Rent::get()?;
            let lamports = rent.minimum_balance(UserLogState::LEN);

            invoke_signed(
                &system_instruction::create_account(
                    user.key,
                    user_log_account.key,
                    lamports,
                    UserLogState::LEN as u64,
                    program_id,
                ),
                &[user.clone(), user_log_account.clone(), system_program.clone()],
                &[&[USER_LOG_SEED, protocol_id.as_ref(), wallet.as_ref(), &[bump]]],
            )?;
        } else {
            return Err(ProgramError::InvalidAccountOwner);
        }

        let new_state = UserLogState {
            is_initialized: true,
            wallet,
            protocol_id,
            last_hash: [0u8; 32],
            event_count: 0,
            migrated_to: None,
            is_migrated: false,
        };

        let mut data = user_log_account.try_borrow_mut_data()?;
        new_state.serialize(&mut &mut data[..])?;

        Ok(())
    }

    fn process_append_activity_event(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        wallet: Pubkey,
        protocol_id: Pubkey,
        event_type: u8,
        magnitude: u64,
        timestamp: i64,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let user_log_account = next_account_info(accounts_iter)?;
        let activity_event_account = next_account_info(accounts_iter)?;
        let protocol_authority = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        if !verify_signer(protocol_authority) {
            return Err(GulliantError::MissingProtocolSignature.into());
        }

        let (expected_user_log_pda, _) = Pubkey::find_program_address(
            &[USER_LOG_SEED, protocol_id.as_ref(), wallet.as_ref()],
            program_id,
        );
        if expected_user_log_pda != *user_log_account.key {
            return Err(ProgramError::InvalidAccountData);
        }
        if user_log_account.owner != program_id {
            return Err(ProgramError::UninitializedAccount);
        }

        let user_log_data = user_log_account.try_borrow_data()?;
        let mut user_log_slice: &[u8] = &user_log_data;
        let mut user_log = UserLogState::deserialize(&mut user_log_slice)?;
        if !user_log.is_initialized {
            return Err(ProgramError::UninitializedAccount);
        }
        if user_log.is_migrated {
            return Err(GulliantError::WalletAlreadyMigrated.into());
        }
        drop(user_log_data);

        let prev_hash = user_log.last_hash;
        let new_hash =
            compute_event_hash(&wallet, &protocol_id, event_type, magnitude, timestamp, &prev_hash);
        let new_index = user_log.event_count;

        let index_bytes = new_index.to_le_bytes();
        let (expected_event_pda, bump) = Pubkey::find_program_address(
            &[ACTIVITY_EVENT_SEED, protocol_id.as_ref(), wallet.as_ref(), &index_bytes],
            program_id,
        );
        if expected_event_pda != *activity_event_account.key {
            return Err(ProgramError::InvalidAccountData);
        }

        if activity_event_account.owner == program_id {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        if *activity_event_account.owner != system_program::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if activity_event_account.data_len() > 0 {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        let rent = solana_program::rent::Rent::get()?;
        let lamports = rent.minimum_balance(ActivityEventAccount::LEN);
        invoke_signed(
            &system_instruction::create_account(
                protocol_authority.key,
                activity_event_account.key,
                lamports,
                ActivityEventAccount::LEN as u64,
                program_id,
            ),
            &[
                protocol_authority.clone(),
                activity_event_account.clone(),
                system_program.clone(),
            ],
            &[&[
                ACTIVITY_EVENT_SEED,
                protocol_id.as_ref(),
                wallet.as_ref(),
                &index_bytes,
                &[bump],
            ]],
        )?;

        let event = ActivityEvent {
            wallet,
            protocol_id,
            event_type,
            magnitude,
            timestamp,
            prev_hash,
            hash: new_hash,
        };

        let event_account_data = ActivityEventAccount {
            index: new_index,
            event,
        };

        let mut event_data = activity_event_account.try_borrow_mut_data()?;
        event_account_data.serialize(&mut &mut event_data[..])?;

        user_log.last_hash = new_hash;
        user_log.event_count += 1;

        let mut user_log_data_mut = user_log_account.try_borrow_mut_data()?;
        user_log.serialize(&mut &mut user_log_data_mut[..])?;

        Ok(())
    }

    fn process_authorize_export(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        old_wallet: Pubkey,
        new_wallet: Pubkey,
        protocol_id: Pubkey,
        authorized_until: i64,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let user_log_account = next_account_info(accounts_iter)?;
        let export_auth_account = next_account_info(accounts_iter)?;
        let protocol_authority = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        if !verify_signer(protocol_authority) {
            return Err(GulliantError::MissingProtocolSignature.into());
        }

        let (expected_user_log_pda, _) = Pubkey::find_program_address(
            &[USER_LOG_SEED, protocol_id.as_ref(), old_wallet.as_ref()],
            program_id,
        );
        if expected_user_log_pda != *user_log_account.key {
            return Err(ProgramError::InvalidAccountData);
        }
        if user_log_account.owner != program_id {
            return Err(ProgramError::UninitializedAccount);
        }

        let user_log_data = user_log_account.try_borrow_data()?;
        let mut user_log_slice: &[u8] = &user_log_data;
        let user_log = UserLogState::deserialize(&mut user_log_slice)?;
        if !user_log.is_initialized {
            return Err(ProgramError::UninitializedAccount);
        }
        if user_log.is_migrated {
            return Err(GulliantError::WalletAlreadyMigrated.into());
        }
        let snapshot_hash = user_log.last_hash;
        drop(user_log_data);

        let (expected_auth_pda, bump) = Pubkey::find_program_address(
            &[
                EXPORT_AUTH_SEED,
                old_wallet.as_ref(),
                new_wallet.as_ref(),
                protocol_id.as_ref(),
            ],
            program_id,
        );
        if expected_auth_pda != *export_auth_account.key {
            return Err(ProgramError::InvalidAccountData);
        }

        if export_auth_account.owner == program_id {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        if *export_auth_account.owner != system_program::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if export_auth_account.data_len() > 0 {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        let rent = solana_program::rent::Rent::get()?;
        let lamports = rent.minimum_balance(ExportAuthorizationState::LEN);
        invoke_signed(
            &system_instruction::create_account(
                protocol_authority.key,
                export_auth_account.key,
                lamports,
                ExportAuthorizationState::LEN as u64,
                program_id,
            ),
            &[
                protocol_authority.clone(),
                export_auth_account.clone(),
                system_program.clone(),
            ],
            &[&[
                EXPORT_AUTH_SEED,
                old_wallet.as_ref(),
                new_wallet.as_ref(),
                protocol_id.as_ref(),
                &[bump],
            ]],
        )?;

        let auth = ExportAuthorization {
            old_wallet,
            new_wallet,
            protocol_id,
            authorized_until,
            log_snapshot_hash: snapshot_hash,
            signer: *protocol_authority.key,
        };

        let auth_state = ExportAuthorizationState {
            is_initialized: true,
            auth,
            used: false,
        };

        let mut auth_data = export_auth_account.try_borrow_mut_data()?;
        auth_state.serialize(&mut &mut auth_data[..])?;

        Ok(())
    }

    fn process_migrate_state(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        old_wallet: Pubkey,
        new_wallet: Pubkey,
        protocol_id: Pubkey,
        current_timestamp: i64,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let old_user_log_account = next_account_info(accounts_iter)?;
        let new_user_log_account = next_account_info(accounts_iter)?;
        let export_auth_account = next_account_info(accounts_iter)?;
        let migrated_link_account = next_account_info(accounts_iter)?;
        let old_wallet_signer = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        if !verify_signer(old_wallet_signer) {
            return Err(GulliantError::MissingUserSignature.into());
        }
        if old_wallet_signer.key != &old_wallet {
            return Err(GulliantError::MissingUserSignature.into());
        }

        let (expected_old_pda, _) = Pubkey::find_program_address(
            &[USER_LOG_SEED, protocol_id.as_ref(), old_wallet.as_ref()],
            program_id,
        );
        if expected_old_pda != *old_user_log_account.key {
            return Err(ProgramError::InvalidAccountData);
        }
        if old_user_log_account.owner != program_id {
            return Err(ProgramError::UninitializedAccount);
        }

        let old_log_data = old_user_log_account.try_borrow_data()?;
        let mut old_log_slice: &[u8] = &old_log_data;
        let old_log = UserLogState::deserialize(&mut old_log_slice)?;
        if !old_log.is_initialized {
            return Err(ProgramError::UninitializedAccount);
        }
        if old_log.is_migrated {
            return Err(GulliantError::WalletAlreadyMigrated.into());
        }
        drop(old_log_data);

        let (expected_auth_pda, _) = Pubkey::find_program_address(
            &[
                EXPORT_AUTH_SEED,
                old_wallet.as_ref(),
                new_wallet.as_ref(),
                protocol_id.as_ref(),
            ],
            program_id,
        );
        if expected_auth_pda != *export_auth_account.key {
            return Err(ProgramError::InvalidAccountData);
        }
        if export_auth_account.owner != program_id {
            return Err(ProgramError::UninitializedAccount);
        }

        let auth_data = export_auth_account.try_borrow_data()?;
        let mut auth_slice: &[u8] = &auth_data;
        let mut auth_state = ExportAuthorizationState::deserialize(&mut auth_slice)?;
        if !auth_state.is_initialized {
            return Err(ProgramError::UninitializedAccount);
        }
        if auth_state.used {
            return Err(GulliantError::ExportAuthorizationAlreadyUsed.into());
        }
        if current_timestamp > auth_state.auth.authorized_until {
            return Err(GulliantError::ExportAuthorizationExpired.into());
        }
        if auth_state.auth.old_wallet != old_wallet
            || auth_state.auth.new_wallet != new_wallet
            || auth_state.auth.protocol_id != protocol_id
        {
            return Err(GulliantError::InvalidWalletPair.into());
        }
        if auth_state.auth.log_snapshot_hash != old_log.last_hash {
            return Err(GulliantError::SnapshotHashMismatch.into());
        }
        drop(auth_data);

        let (expected_new_pda, bump_new) = Pubkey::find_program_address(
            &[USER_LOG_SEED, protocol_id.as_ref(), new_wallet.as_ref()],
            program_id,
        );
        if expected_new_pda != *new_user_log_account.key {
            return Err(ProgramError::InvalidAccountData);
        }

        if *new_user_log_account.owner == system_program::ID && new_user_log_account.data_len() == 0
        {
            let rent = solana_program::rent::Rent::get()?;
            let lamports = rent.minimum_balance(UserLogState::LEN);
            invoke_signed(
                &system_instruction::create_account(
                    old_wallet_signer.key,
                    new_user_log_account.key,
                    lamports,
                    UserLogState::LEN as u64,
                    program_id,
                ),
                &[
                    old_wallet_signer.clone(),
                    new_user_log_account.clone(),
                    system_program.clone(),
                ],
                &[&[
                    USER_LOG_SEED,
                    protocol_id.as_ref(),
                    new_wallet.as_ref(),
                    &[bump_new],
                ]],
            )?;
        } else if new_user_log_account.owner != program_id {
            return Err(ProgramError::InvalidAccountOwner);
        } else {
            let new_log_data = new_user_log_account.try_borrow_data()?;
            let mut new_log_slice: &[u8] = &new_log_data;
            let existing_new_log = UserLogState::deserialize(&mut new_log_slice)?;
            if existing_new_log.is_migrated {
                return Err(GulliantError::WalletAlreadyMigrated.into());
            }
            drop(new_log_data);
        }

        let new_log_state = UserLogState {
            is_initialized: true,
            wallet: new_wallet,
            protocol_id,
            last_hash: old_log.last_hash,
            event_count: old_log.event_count,
            migrated_to: None,
            is_migrated: false,
        };

        let mut new_log_data_mut = new_user_log_account.try_borrow_mut_data()?;
        new_log_state.serialize(&mut &mut new_log_data_mut[..])?;
        drop(new_log_data_mut);

        let mut old_log_mut = old_log;
        old_log_mut.is_migrated = true;
        old_log_mut.migrated_to = Some(new_wallet);

        let mut old_log_data_mut = old_user_log_account.try_borrow_mut_data()?;
        old_log_mut.serialize(&mut &mut old_log_data_mut[..])?;
        drop(old_log_data_mut);

        auth_state.used = true;
        let mut auth_data_mut = export_auth_account.try_borrow_mut_data()?;
        auth_state.serialize(&mut &mut auth_data_mut[..])?;
        drop(auth_data_mut);

        let (expected_link_pda, bump_link) = Pubkey::find_program_address(
            &[
                MIGRATED_LINK_SEED,
                old_wallet.as_ref(),
                new_wallet.as_ref(),
                protocol_id.as_ref(),
            ],
            program_id,
        );
        if expected_link_pda != *migrated_link_account.key {
            return Err(ProgramError::InvalidAccountData);
        }

        if *migrated_link_account.owner == system_program::ID
            && migrated_link_account.data_len() == 0
        {
            let rent = solana_program::rent::Rent::get()?;
            let lamports = rent.minimum_balance(MigratedStateLink::LEN);
            invoke_signed(
                &system_instruction::create_account(
                    old_wallet_signer.key,
                    migrated_link_account.key,
                    lamports,
                    MigratedStateLink::LEN as u64,
                    program_id,
                ),
                &[
                    old_wallet_signer.clone(),
                    migrated_link_account.clone(),
                    system_program.clone(),
                ],
                &[&[
                    MIGRATED_LINK_SEED,
                    old_wallet.as_ref(),
                    new_wallet.as_ref(),
                    protocol_id.as_ref(),
                    &[bump_link],
                ]],
            )?;
        } else if migrated_link_account.owner != program_id {
            return Err(ProgramError::InvalidAccountOwner);
        }

        let link = MigratedStateLink {
            old_wallet,
            new_wallet,
            protocol_id,
            snapshot_hash: old_log_mut.last_hash,
            migrated_at: current_timestamp,
        };

        let mut link_data = migrated_link_account.try_borrow_mut_data()?;
        link.serialize(&mut &mut link_data[..])?;

        Ok(())
    }
}

