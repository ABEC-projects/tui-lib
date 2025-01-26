#[derive(Debug, thiserror::Error)]
pub enum TtyCreationError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Terminfo database not found on the machine.")]
    TerminfoDbNotFound,
    #[error("Error while processing terminfo entry.")]
    TerminfoProcessingError,
}

impl From<nix::errno::Errno> for TtyCreationError {
    fn from(value: nix::errno::Errno) -> Self {
        Self::IoError(value.into())
    }
}

impl From<terminfo::Error> for TtyCreationError {
    fn from(value: terminfo::Error) -> Self {
        use terminfo::Error as Te;
        match value {
            Te::Io(io_error) => Self::IoError(io_error),
            Te::NotFound => Self::TerminfoDbNotFound,
            Te::Parse | Te::Expand(_) => Self::TerminfoProcessingError,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TtyCapabilityError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Could not find capability `{cap_name}` in terminfo database.")]
    CapabilityNotFound {
        cap_name: String,
    },
    #[error("Failed to expand capability from terminfo database.")]
    CapabilityExpansionError,
}

impl From<nix::errno::Errno> for TtyCapabilityError {
    fn from(value: nix::errno::Errno) -> Self {
        Self::IoError(value.into())
    }
}
