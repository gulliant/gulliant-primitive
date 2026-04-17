use solana_program::program_error::ProgramError;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GulliantError {
    MissingUserSignature = 0,
    MissingProtocolSignature = 1,
    WalletAlreadyMigrated = 2,
    ExportAuthorizationAlreadyUsed = 3,
    ExportAuthorizationExpired = 4,
    SnapshotHashMismatch = 5,
    InvalidWalletPair = 6,
    InvalidProtocolAuthority = 7,
    ProtocolConfigAlreadyInitialized = 8,
    LogIntegrityFailure = 9,
}

impl From<GulliantError> for ProgramError {
    fn from(e: GulliantError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
