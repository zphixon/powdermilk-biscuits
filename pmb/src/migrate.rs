//! PMB versions
//!
//! Perhaps there are some architectural changes that should take place to make this not so silly,
//! but I wanted to be able to do `bincode::encode()/decode()` directly on [State] and friends, so
//! now you (most likely future me) have (has) to live with this for the moment. Anyway. For the
//! most part, changes can be made to types which are written to disk as long as they have a
//! `#[disk_skip]` attribute. If you ever want to make a change involving a field without
//! `#[disk_skip]`, you need to do these things:
//!
//! - Increment [Version::CURRENT]
//! - Add a new module named v\[old\] where \[old\] is the previous version
//! - Copy/paste every type with `derive(Disk)` to that module, suffixing its name with V\[new\], and
//!   removing any fields that do not get serialized to disk
//! - Derive bincode::Decode for each type
//! - Add the new version to [Version::upgrade_type] and edit the compatibility between the old and
//!   new [State]s
//! - Add the new version to [from], replacing the old types from the new version to the previous
//!   version and fight for your life

use crate::{
    error::{ErrorKind, PmbError},
    Backend, State, StrokeBackend,
};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    io::Read,
    path::Path,
};

#[derive(Debug)]
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
    pub const CURRENT: Self = Version(4);

    pub fn upgrade_type(from: Self) -> UpgradeType {
        use UpgradeType::*;

        if from == Self::CURRENT {
            return Smooth;
        }

        match from {
            Version(3) => Smooth,
            Version(2) => Smooth,
            Version(1) => Smooth,
            _ => Incompatible,
        }
    }
}

pub fn from<B, S>(version: Version, path: impl AsRef<Path>) -> Result<State<B, S>, PmbError>
where
    B: Backend,
    S: StrokeBackend,
{
    log::info!(
        "upgrading from {} to {} is {:?}",
        version,
        Version::CURRENT,
        Version::upgrade_type(version)
    );

    use crate::stroke::*;
    let file = std::fs::File::open(&path)?;

    match version {
        version if version == Version::CURRENT => unreachable!(),

        Version(3) => {
            let v3: v3::StateV3 = v3::read(file)?.into();

            let state = State {
                strokes: v3
                    .strokes
                    .into_iter()
                    .map(|v3| Stroke {
                        points: {
                            v3.points
                                .iter()
                                .zip(v3.pressure.iter())
                                .map(|(point, &pressure)| StrokeElement {
                                    x: point.x,
                                    y: point.y,
                                    pressure,
                                })
                                .collect()
                        },
                        color: v3.color,
                        brush_size: v3.brush_size,
                        erased: v3.erased,
                        ..Default::default()
                    })
                    .collect(),
                brush_size: v3.brush_size,
                zoom: v3.zoom,
                ..Default::default()
            };

            return Ok(state);
        }

        Version(2) => {
            let v2: v2::StateV2 = v2::read(file)?.into();

            let state = State {
                strokes: v2
                    .strokes
                    .into_iter()
                    .map(|v2| Stroke {
                        points: v2
                            .points
                            .iter()
                            .map(|v2| StrokeElement {
                                x: v2.x,
                                y: v2.y,
                                pressure: v2.pressure,
                            })
                            .collect(),
                        color: v2.color,
                        brush_size: v2.brush_size,
                        erased: v2.erased,
                        ..Default::default()
                    })
                    .collect(),
                brush_size: v2.brush_size,
                zoom: v2.zoom,
                ..Default::default()
            };

            return Ok(state);
        }

        Version(1) => {
            let v1: v1::StateV1 = v1::read(file)?.into();

            let state = State {
                strokes: v1
                    .strokes
                    .into_iter()
                    .map(|v1| Stroke {
                        points: v1
                            .points
                            .iter()
                            .map(|v1| StrokeElement {
                                x: v1.x,
                                y: v1.y,
                                pressure: v1.pressure,
                            })
                            .collect(),
                        color: v1.color,
                        brush_size: v1.brush_size,
                        erased: v1.erased,
                        ..Default::default()
                    })
                    .collect(),
                brush_size: v1.brush_size,
                zoom: v1.zoom,
                ..Default::default()
            };

            return Ok(state);
        }

        _ => Err(PmbError::new(ErrorKind::UnknownVersion(version))),
    }
}

mod v3 {
    use super::*;
    use bincode::config::standard;

    #[derive(bincode::Decode)]
    pub struct StrokePointV3 {
        pub x: f32,
        pub y: f32,
    }

    #[derive(bincode::Decode)]
    pub struct StrokeV3 {
        pub points: Vec<StrokePointV3>,
        pub pressure: Vec<f32>,
        pub color: [u8; 3],
        pub brush_size: f32,
        pub erased: bool,
    }

    #[derive(bincode::Decode)]
    pub struct StateV3 {
        pub strokes: Vec<StrokeV3>,
        pub brush_size: usize,
        pub zoom: f32,
        pub origin: StrokePointV3,
    }

    pub fn read(mut reader: impl Read) -> Result<StateV3, PmbError> {
        let mut magic = [0; 3];
        reader.read_exact(&mut magic)?;

        if magic != crate::PMB_MAGIC {
            return Err(PmbError::new(ErrorKind::MissingHeader));
        }

        let mut version_bytes = [0; std::mem::size_of::<u64>()];
        reader.read_exact(&mut version_bytes)?;
        let version = Version(u64::from_le_bytes(version_bytes));

        if version != Version(3) {
            return Err(PmbError::new(ErrorKind::VersionMismatch(version)));
        }

        let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
        Ok(bincode::decode_from_std_read(
            &mut deflate_reader,
            standard(),
        )?)
    }
}

mod v2 {
    use super::*;
    use bincode::config::standard;

    #[derive(bincode::Decode)]
    pub struct StrokePointV2 {
        pub x: f32,
        pub y: f32,
    }

    #[derive(bincode::Decode)]
    #[repr(C)]
    pub struct StrokeElementV2 {
        pub x: f32,
        pub y: f32,
        pub pressure: f32,
    }

    #[derive(bincode::Decode)]
    pub struct StrokeV2 {
        pub points: Vec<StrokeElementV2>,
        pub color: [u8; 3],
        pub brush_size: f32,
        pub erased: bool,
    }

    #[derive(bincode::Decode)]
    pub struct StateV2 {
        pub strokes: Vec<StrokeV2>,
        pub brush_size: usize,
        pub zoom: f32,
        pub origin: StrokePointV2,
    }

    pub fn read(mut reader: impl Read) -> Result<StateV2, PmbError> {
        let mut magic = [0; 3];
        reader.read_exact(&mut magic)?;

        if magic != crate::PMB_MAGIC {
            return Err(PmbError::new(ErrorKind::MissingHeader));
        }

        let mut version_bytes = [0; std::mem::size_of::<u64>()];
        reader.read_exact(&mut version_bytes)?;
        let version = Version(u64::from_le_bytes(version_bytes));

        if version != Version(2) {
            return Err(PmbError::new(ErrorKind::VersionMismatch(version)));
        }

        let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
        Ok(bincode::decode_from_std_read(
            &mut deflate_reader,
            standard(),
        )?)
    }
}

mod v1 {
    use super::*;
    use bincode::config::standard;

    #[derive(bincode::Decode)]
    pub struct StrokePoint {
        pub x: f32,
        pub y: f32,
    }

    #[derive(bincode::Decode)]
    #[repr(C)]
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
