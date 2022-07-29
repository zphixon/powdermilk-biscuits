#[derive(Debug)]
pub enum PmbError {
    MissingHeader,
    IoError(std::io::Error),
    BincodeError(Box<dyn std::error::Error>),
}

impl From<std::io::Error> for PmbError {
    fn from(err: std::io::Error) -> Self {
        PmbError::IoError(err)
    }
}

impl From<bincode::error::DecodeError> for PmbError {
    fn from(err: bincode::error::DecodeError) -> Self {
        PmbError::BincodeError(Box::new(err))
    }
}

impl From<bincode::error::EncodeError> for PmbError {
    fn from(err: bincode::error::EncodeError) -> Self {
        PmbError::BincodeError(Box::new(err))
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
