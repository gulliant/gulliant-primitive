use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ProtocolConfig {
    pub is_initialized: bool,
    pub protocol_id: Pubkey,
    pub authority: Pubkey,
}

impl ProtocolConfig {
    pub const LEN: usize = 1 + 32 + 32;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct UserLogState {
    pub is_initialized: bool,
    pub wallet: Pubkey,
    pub protocol_id: Pubkey,
    pub last_hash: [u8; 32],
    pub event_count: u64,
    pub migrated_to: Option<Pubkey>,
    pub is_migrated: bool,
}

impl UserLogState {
    pub const LEN: usize = 1 + 32 + 32 + 32 + 8 + (1 + 32) + 1;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ActivityEvent {
    pub wallet: Pubkey,
    pub protocol_id: Pubkey,
    pub event_type: u8,
    pub magnitude: u64,
    pub timestamp: i64,
    pub prev_hash: [u8; 32],
    pub hash: [u8; 32],
}

impl ActivityEvent {
    pub const LEN: usize = 32 + 32 + 1 + 8 + 8 + 32 + 32;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ActivityEventAccount {
    pub index: u64,
    pub event: ActivityEvent,
}

impl ActivityEventAccount {
    pub const LEN: usize = 8 + ActivityEvent::LEN;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ExportAuthorization {
    pub old_wallet: Pubkey,
    pub new_wallet: Pubkey,
    pub protocol_id: Pubkey,
    pub authorized_until: i64,
    pub log_snapshot_hash: [u8; 32],
    pub signer: Pubkey,
}

impl ExportAuthorization {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 32 + 32;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ExportAuthorizationState {
    pub is_initialized: bool,
    pub auth: ExportAuthorization,
    pub used: bool,
}

impl ExportAuthorizationState {
    pub const LEN: usize = 1 + ExportAuthorization::LEN + 1;
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MigratedStateLink {
    pub old_wallet: Pubkey,
    pub new_wallet: Pubkey,
    pub protocol_id: Pubkey,
    pub snapshot_hash: [u8; 32],
    pub migrated_at: i64,
}

impl MigratedStateLink {
    pub const LEN: usize = 32 + 32 + 32 + 32 + 8;
}

