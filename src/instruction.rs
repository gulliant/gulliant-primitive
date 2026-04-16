use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone)]
pub enum GulliantInstruction {
    InitializeUserLog {
        wallet: Pubkey,
        protocol_id: Pubkey,
    },
    AppendActivityEvent {
        wallet: Pubkey,
        protocol_id: Pubkey,
        event_type: u8,
        magnitude: u64,
        timestamp: i64,
    },
    AuthorizeExport {
        old_wallet: Pubkey,
        new_wallet: Pubkey,
        protocol_id: Pubkey,
        authorized_until: i64,
    },
    MigrateState {
        old_wallet: Pubkey,
        new_wallet: Pubkey,
        protocol_id: Pubkey,
        current_timestamp: i64,
    },
}