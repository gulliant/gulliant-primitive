use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};

solana_program::declare_id!("FSTYFLyyyUAVGz5bak4waMr29gEawSwKeDgw5n1KBZhi");

const STATE_SEED: &[u8] = b"naive_character_state";

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct NaiveCharacterState {
    pub is_initialized: bool,
    pub character_owner: Pubkey,
    pub matchmaking_tier: u8,
    pub permissions_level: u8, // 0=none, 1=basic, 2=veteran
}

impl NaiveCharacterState {
    pub const LEN: usize = 1 + 32 + 1 + 1;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum NaiveInstruction {
    Initialize {
        seller: Pubkey,
        matchmaking_tier: u8,
        permissions_level: u8,
    },
    Transfer {
        seller: Pubkey,
        new_owner: Pubkey,
    },
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = NaiveInstruction::try_from_slice(input)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        NaiveInstruction::Initialize {
            seller,
            matchmaking_tier,
            permissions_level,
        } => process_initialize(
            program_id,
            accounts,
            seller,
            matchmaking_tier,
            permissions_level,
        ),
        NaiveInstruction::Transfer { seller, new_owner } => {
            process_transfer(program_id, accounts, seller, new_owner)
        }
    }
}

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    seller: Pubkey,
    matchmaking_tier: u8,
    permissions_level: u8,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let state_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program_account = next_account_info(accounts_iter)?;

    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if *system_program_account.key != system_program::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    let (expected_pda, bump) =
        Pubkey::find_program_address(&[STATE_SEED, seller.as_ref()], program_id);

    if expected_pda != *state_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    if *state_account.owner != system_program::ID || state_account.data_len() > 0 {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(NaiveCharacterState::LEN);

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            state_account.key,
            lamports,
            NaiveCharacterState::LEN as u64,
            program_id,
        ),
        &[
            payer.clone(),
            state_account.clone(),
            system_program_account.clone(),
        ],
        &[&[STATE_SEED, seller.as_ref(), &[bump]]],
    )?;

    let state = NaiveCharacterState {
        is_initialized: true,
        character_owner: seller,
        matchmaking_tier,
        permissions_level,
    };

    let mut data = state_account.try_borrow_mut_data()?;
    state.serialize(&mut &mut data[..])?;

    msg!("==========");
    msg!("NAIVE INIT");
    msg!("owner: {}", seller);
    msg!("matchmaking_tier: {}", matchmaking_tier);
    msg!("permissions_level: {}", permissions_level);

    Ok(())
}

fn process_transfer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    seller: Pubkey,
    new_owner: Pubkey,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let state_account = next_account_info(accounts_iter)?;
    let current_owner = next_account_info(accounts_iter)?;

    if !current_owner.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_pda, _) =
        Pubkey::find_program_address(&[STATE_SEED, seller.as_ref()], program_id);

    if expected_pda != *state_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    if state_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut data = state_account.try_borrow_mut_data()?;
    let mut state = NaiveCharacterState::try_from_slice(&data)?;

    if !state.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }

    if state.character_owner != *current_owner.key {
        return Err(ProgramError::IllegalOwner);
    }

    msg!("==============");
    msg!("NAIVE TRANSFER");
    msg!("before_owner: {}", state.character_owner);
    msg!("before_matchmaking_tier: {}", state.matchmaking_tier);
    msg!("before_permissions_level: {}", state.permissions_level);

    // Intentionally naive:
    // owner changes, but player-bound state remains attached to the asset state
    state.character_owner = new_owner;

    state.serialize(&mut &mut data[..])?;

    msg!("after_owner: {}", new_owner);
    msg!("after_matchmaking_tier: {}", state.matchmaking_tier);
    msg!("after_permissions_level: {}", state.permissions_level);
    msg!("WARNING: state moved with asset");

    Ok(())
}
