use crate::migrate::Version;
use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

pub trait PmbErrorExt {
    fn display(self);
    fn display_with(self, header: String);
    fn problem(self, why: String) -> Self;
}

impl<T> PmbErrorExt for Result<T, PmbError> {
    fn display(self) {
        if let Err(err) = self {
            err.display();
        }
    }

    fn display_with(self, header: String) {
        if let Err(err) = self {
            err.display_with(header);
        }
    }

    fn problem(self, why: String) -> Self {
        if matches!(self, Err(_)) {
            Err(self.err().unwrap().problem(why))
        } else {
            self
        }
    }
}

impl PmbErrorExt for PmbError {
    fn display(self) {
        let msg = self
            .why
            .iter()
            .rev()
            .fold(String::new(), |acc, why| format!("{why} {acc}"));
        crate::ui::error(&msg);
    }

    fn display_with(self, header: String) {
        let msg = self
            .why
            .iter()
            .rev()
            .fold(String::new(), |acc, why| format!("{why} {acc}"));
        crate::ui::error(&format!("{header}\n\n{msg}"));
    }

    fn problem(mut self, why: String) -> Self {
        self.why.push(why);
        self
    }
}

#[derive(Debug)]
pub struct PmbError {
    pub kind: ErrorKind,
    why: Vec<String>,
}

// SAFETY: Fine because dyn Error + 'static is Send (right???)
unsafe impl Send for PmbError {}

// SAFETY: Fine because no methods on &PmbError or &PmbErrorKind mutate (probably)
unsafe impl Sync for PmbError {}
// really i just wanna do Version::new()? ant have it work with anyhow,, it's probalby fine;

impl PmbError {
    pub fn new(kind: ErrorKind) -> Self {
        PmbError {
            why: vec![format!("{kind}")],
            kind,
        }
    }
}

impl Display for PmbError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.kind)
    }
}

impl Error for PmbError {
    fn cause(&self) -> Option<&dyn Error> {
        match &self.kind {
            ErrorKind::IoError(err) => Some(err),
            ErrorKind::EncodeDecode(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    MissingHeader,
    IoError(std::io::Error),
    EncodeDecode(Box<dyn std::error::Error + 'static>),
    VersionMismatch(Version),
    UnknownVersion(Version),
    IncompatibleVersion(Version),
    Tessellator(lyon::lyon_tessellation::TessellationError),
}

impl From<std::io::Error> for PmbError {
    fn from(err: std::io::Error) -> Self {
        PmbError::new(ErrorKind::IoError(err))
    }
}

impl From<bincode::error::DecodeError> for PmbError {
    fn from(err: bincode::error::DecodeError) -> Self {
        PmbError::new(ErrorKind::EncodeDecode(Box::new(err)))
    }
}

impl From<bincode::error::EncodeError> for PmbError {
    fn from(err: bincode::error::EncodeError) -> Self {
        PmbError::new(ErrorKind::EncodeDecode(Box::new(err)))
    }
}

impl From<ron::error::SpannedError> for PmbError {
    fn from(err: ron::error::SpannedError) -> Self {
        PmbError::new(ErrorKind::EncodeDecode(Box::new(err)))
    }
}

impl From<lyon::lyon_tessellation::TessellationError> for PmbError {
    fn from(err: lyon::lyon_tessellation::TessellationError) -> Self {
        PmbError::new(ErrorKind::Tessellator(err))
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            ErrorKind::MissingHeader => write!(f, "Missing PMB header"),
            ErrorKind::IoError(err) => write!(f, "{err}"),
            ErrorKind::EncodeDecode(err) => write!(f, "{err}"),
            ErrorKind::VersionMismatch(version) => {
                write!(f, "Expected version {}, got {version}", Version::CURRENT)
            }
            ErrorKind::UnknownVersion(version) => {
                write!(f, "Unknown version number {version}")
            }
            ErrorKind::IncompatibleVersion(version) => {
                write!(
                    f,
                    "Version {version} is incompatible with the current version {}",
                    Version::CURRENT
                )
            }
            ErrorKind::Tessellator(err) => {
                write!(f, "Tessellator error: {}", err)
            }
        }
    }
}
