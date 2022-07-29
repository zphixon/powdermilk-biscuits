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
            use crate::stroke::*;

            let file = std::fs::File::open(&path)?;
            let v1: v1::StateV1 = v1::read(file)?.into();

            let mut state = State {
                strokes: v1
                    .strokes
                    .into_iter()
                    .map(|v1| Stroke {
                        points: v1
                            .points
                            .into_iter()
                            .map(|v1| {
                                let x = v1.x;
                                let y = v1.y;
                                let pressure = v1.pressure;
                                StrokeElement { x, y, pressure }
                            })
                            .collect(),
                        color: v1.color,
                        brush_size: v1.brush_size,
                        erased: v1.erased,
                        spline: None,
                        backend: None,
                    })
                    .collect(),
                brush_size: v1.brush_size,
                zoom: v1.zoom,
                ..Default::default()
            };

            state.strokes.iter_mut().for_each(Stroke::calculate_spline);

            return Ok(state);
        }

        _ => Err(PmbError::new(ErrorKind::UnknownVersion(version))),
    }
}

mod v1 {
    use super::*;
    use bincode::config::standard;
    use std::io::Read;

    #[derive(bincode::Decode)]
    pub struct StrokePoint {
        pub x: f32,
        pub y: f32,
    }

    #[derive(bincode::Decode)]
    #[repr(packed)]
    pub struct StrokeElement {
        pub x: f32,
        pub y: f32,
        pub pressure: f32,
    }

    #[derive(bincode::Decode)]
    pub struct StrokeV1 {
        pub points: Vec<StrokeElement>,
        pub color: [u8; 3],
        pub brush_size: f32,
        pub erased: bool,
    }

    #[derive(bincode::Decode)]
    pub struct StateV1 {
        pub strokes: Vec<StrokeV1>,
        pub brush_size: usize,
        pub zoom: f32,
        pub origin: StrokePoint,
    }

    pub fn read(mut reader: impl Read) -> Result<StateV1, PmbError> {
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
