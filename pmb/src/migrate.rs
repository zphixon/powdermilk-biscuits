use crate::{
    error::{ErrorKind, PmbError},
    Backend, State, StrokeBackend,
};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    path::Path,
};

pub enum UpgradeType {
    Smooth,
    Rocky,
    Incompatible,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Version(pub u64);

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl Version {
    pub const CURRENT: Self = Version(2);

    pub fn upgrade_type(from: Self) -> UpgradeType {
        use UpgradeType::*;

        if from == Self::CURRENT {
            return Smooth;
        }

        match from {
            Version(1) => Rocky,
            _ => Incompatible,
        }
    }
}

pub fn from<B, S>(version: Version, path: impl AsRef<Path>) -> Result<State<B, S>, PmbError>
where
    B: Backend,
    S: StrokeBackend,
{
    match version {
        version if version == Version::CURRENT => unreachable!(),

        Version(1) => {
            let mut state: State<B, S> = v1::to_v2(path)?.into();

            state
                .strokes
                .iter_mut()
                .for_each(crate::stroke::Stroke::calculate_spline);

            return Ok(state);
        }

        _ => Err(PmbError::new(ErrorKind::UnknownVersion(version))),
    }
}

mod v1 {
    use super::*;
    use bincode::config::standard;
    use std::io::Read;

    #[repr(transparent)]
    pub struct StateV2<B, S>(State<B, S>)
    where
        B: Backend,
        S: StrokeBackend;

    impl<B: Backend, S: StrokeBackend> bincode::Decode for StateV2<B, S> {
        fn decode<D: bincode::de::Decoder>(
            decoder: &mut D,
        ) -> Result<Self, bincode::error::DecodeError> {
            Ok(StateV2(bincode::Decode::decode(decoder)?))
        }
    }

    impl<B: Backend, S: StrokeBackend> Into<State<B, S>> for StateV2<B, S> {
        fn into(self) -> State<B, S> {
            self.0
        }
    }

    pub fn to_v2<B, S>(path: impl AsRef<Path>) -> Result<StateV2<B, S>, PmbError>
    where
        B: Backend,
        S: StrokeBackend,
    {
        let file = std::fs::File::open(&path)?;
        read_v1(file)
    }

    fn read_v1<B, S>(mut reader: impl Read) -> Result<StateV2<B, S>, PmbError>
    where
        B: Backend,
        S: StrokeBackend,
    {
        let mut magic = [0; 3];
        reader.read_exact(&mut magic)?;

        if magic != crate::PMB_MAGIC {
            return Err(PmbError::new(ErrorKind::MissingHeader));
        }

        let mut version_bytes = [0; std::mem::size_of::<u64>()];
        reader.read_exact(&mut version_bytes)?;
        let version = Version(u64::from_le_bytes(version_bytes));

        if version != Version(1) {
            return Err(PmbError::new(ErrorKind::VersionMismatch(version)));
        }

        let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
        Ok(bincode::decode_from_std_read(
            &mut deflate_reader,
            standard(),
        )?)
    }
}
