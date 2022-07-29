use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

pub trait PmbErrorExt {
    fn display(self);
    fn problem(self, why: String) -> Self;
}

impl<T> PmbErrorExt for Result<T, PmbError> {
    fn display(self) {
        match &self {
            Err(err) => {
                let msg = err
                    .why
                    .iter()
                    .fold(String::new(), |acc, why| format!("{why} {acc}"));
                crate::ui::error(&msg);
            }
            _ => {}
        }
    }

    fn problem(mut self, why: String) -> Self {
        match self.as_mut().err().as_mut() {
            Some(err) => {
                err.why.push(why);
            }
            _ => {}
        }

        self
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

    fn problem(mut self, why: String) -> Self {
        self.why.push(why);
        self
    }
}

#[derive(Debug)]
pub struct PmbError {
    kind: ErrorKind,
    why: Vec<String>,
}

impl PmbError {
    pub fn because(kind: ErrorKind, reason: String) -> Self {
        PmbError {
            kind,
            why: vec![reason],
        }
    }

    pub fn new(kind: ErrorKind) -> Self {
        PmbError {
            kind,
            why: Vec::new(),
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
            ErrorKind::BincodeError(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    MissingHeader,
    IoError(std::io::Error),
    BincodeError(Box<dyn std::error::Error>),
}

impl From<std::io::Error> for PmbError {
    fn from(err: std::io::Error) -> Self {
        let problem = format!("{err}");
        PmbError::new(ErrorKind::IoError(err)).problem(problem)
    }
}

impl From<bincode::error::DecodeError> for PmbError {
    fn from(err: bincode::error::DecodeError) -> Self {
        let problem = format!("{err}");
        PmbError::new(ErrorKind::BincodeError(Box::new(err))).problem(problem)
    }
}

impl From<bincode::error::EncodeError> for PmbError {
    fn from(err: bincode::error::EncodeError) -> Self {
        let problem = format!("{err}");
        PmbError::new(ErrorKind::BincodeError(Box::new(err))).problem(problem)
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            ErrorKind::MissingHeader => write!(f, "Missing PMB header"),
            ErrorKind::IoError(err) => write!(f, "{err}"),
            ErrorKind::BincodeError(err) => write!(f, "{err}"),
        }
    }
}
