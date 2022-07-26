pub type Result<T> = core::result::Result<T, PmbError>;

#[derive(Debug)]
pub enum PmbError {
    MissingHeader,
    IoError(std::io::Error),
    BincodeError(bincode::Error),
}

impl From<std::io::Error> for PmbError {
    fn from(err: std::io::Error) -> Self {
        PmbError::IoError(err)
    }
}

impl From<bincode::Error> for PmbError {
    fn from(err: bincode::Error) -> Self {
        PmbError::BincodeError(err)
    }
}

impl std::fmt::Display for PmbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PmbError::MissingHeader => write!(f, "Missing PMB header"),
            PmbError::IoError(err) => write!(f, "{err}"),
            PmbError::BincodeError(err) => write!(f, "{err}"),
        }
    }
}
