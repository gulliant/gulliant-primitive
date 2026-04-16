use sha2::{Digest, Sha256};
use solana_program::{
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    error::GulliantError,
    state::ActivityEvent,
};

pub fn compute_event_hash(
    wallet: &Pubkey,
    protocol_id: &Pubkey,
    event_type: u8,
    magnitude: u64,
    timestamp: i64,
    prev_hash: &[u8; 32],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(wallet.as_ref());
    hasher.update(protocol_id.as_ref());
    hasher.update(event_type.to_le_bytes());
    hasher.update(magnitude.to_le_bytes());
    hasher.update(timestamp.to_le_bytes());
    hasher.update(prev_hash);
    hasher.finalize().into()
}

pub fn verify_log_chain(events: &[ActivityEvent]) -> Result<(), ProgramError> {
    for i in 0..events.len() {
        let event = &events[i];
        let computed = compute_event_hash(
            &event.wallet,
            &event.protocol_id,
            event.event_type,
            event.magnitude,
            event.timestamp,
            &event.prev_hash,
        );
        if computed != event.hash {
            return Err(GulliantError::LogIntegrityFailure.into());
        }
        if i > 0 && event.prev_hash != events[i - 1].hash {
            return Err(GulliantError::LogIntegrityFailure.into());
        }
    }
    Ok(())
}

pub fn verify_signer(account: &solana_program::account_info::AccountInfo) -> bool {
    account.is_signer
}