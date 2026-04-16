use solana_program::program_error::ProgramError;

#[derive(Debug, Clone, Copy)]
pub enum GulliantError {
    MissingProtocolSignature,
    MissingUserSignature,
    WalletAlreadyMigrated,
    ExportAuthorizationExpired,
    ExportAuthorizationAlreadyUsed,
    SnapshotHashMismatch,
    InvalidWalletPair,
    UnauthorizedProtocol,
    LogIntegrityFailure,
}

impl From<GulliantError> for ProgramError {
    fn from(e: GulliantError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
