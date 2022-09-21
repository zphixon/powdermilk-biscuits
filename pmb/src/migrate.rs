//! PMB versions
//!
//! Perhaps there are some architectural changes that should take place to make this not so silly,
//! but I wanted to be able to do `bincode::encode()/decode()` directly on [Sketch] and friends, so
//! now you (most likely future me) have (has) to live with this for the moment. Anyway. For the
//! most part, changes can be made to types which are written to disk as long as they have a
//! `#[skip]` attribute. If you ever want to make a change involving a field without
//! `#[skip]`, you need to do these things:
//!
//! - Increment [Version::CURRENT]
//! - Add a new module named v\[old\] where \[old\] is the previous version
//! - Copy/paste every type with `derive(Disk)` to that module, suffixing its name with V\[new\], and
//!   removing any fields that do not get serialized to disk
//! - Derive bincode::Decode for each type
//! - Add the new version to [Version::upgrade_type] and edit the compatibility between the old and
//!   new [Sketch]es
//! - Add the new version to [from], replacing the old types from the new version to the previous
//!   version and fight for your life

use crate::{
    error::{ErrorKind, PmbError},
    Sketch, StrokeBackend,
};
use bincode::config::standard;
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    io::{Read, Write},
    path::Path,
};

pub fn read<S: StrokeBackend>(mut reader: impl Read) -> Result<Sketch<S>, PmbError> {
    let mut magic = [0; 3];
    reader.read_exact(&mut magic)?;

    if magic != crate::PMB_MAGIC {
        return Err(PmbError::new(ErrorKind::MissingHeader));
    }

    let mut version_bytes = [0; std::mem::size_of::<u64>()];
    reader.read_exact(&mut version_bytes)?;
    let version = Version(u64::from_le_bytes(version_bytes));

    log::debug!("got version {}", version);
    if version != Version::CURRENT {
        return Err(PmbError::new(ErrorKind::VersionMismatch(version)));
    }

    log::debug!("inflating");
    let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
    Ok(bincode::decode_from_std_read(
        &mut deflate_reader,
        bincode::config::standard(),
    )?)
}

pub fn write<S: StrokeBackend>(
    path: impl AsRef<std::path::Path>,
    state: &Sketch<S>,
) -> Result<(), PmbError> {
    log::debug!("truncating {} and deflating", path.as_ref().display());

    let mut file = std::fs::File::create(&path)?;
    file.write_all(&crate::PMB_MAGIC)?;
    file.write_all(&u64::to_le_bytes(Version::CURRENT.0))?;

    let mut deflate_writer = flate2::write::DeflateEncoder::new(file, flate2::Compression::fast());
    bincode::encode_into_std_write(state, &mut deflate_writer, bincode::config::standard())?;

    Ok(())
}

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
    pub const CURRENT: Self = Version(6);

    pub fn upgrade_type(from: Self) -> UpgradeType {
        use UpgradeType::*;

        if from == Self::CURRENT {
            return Smooth;
        }

        match from {
            Version(5) => Smooth,
            Version(4) => Rocky,
            Version(3) => Smooth,
            Version(2) => Smooth,
            Version(1) => Smooth,
            _ => Incompatible,
        }
    }
}

#[allow(clippy::needless_return)]
pub fn from<S>(version: Version, path: impl AsRef<Path>) -> Result<Sketch<S>, PmbError>
where
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

        Version(5) => {
            let v5: v5::SketchV5 = v5::read(file)?;

            let state = Sketch {
                strokes: crate::map_from_vec(
                    v5.strokes
                        .into_iter()
                        .map(|v5| Stroke {
                            points: {
                                v5.points
                                    .iter()
                                    .map(|point| StrokeElement {
                                        x: point.x,
                                        y: point.y,
                                        pressure: point.pressure,
                                    })
                                    .collect()
                            },
                            color: v5.color,
                            brush_size: v5.brush_size,
                            erased: v5.erased,
                            ..Default::default()
                        })
                        .collect(),
                ),
                zoom: v5.zoom,
                ..Default::default()
            };

            return Ok(state);
        }

        Version(4) => {
            let v4: v4::StateV4 = v4::read(file)?;

            let state = Sketch {
                strokes: crate::map_from_vec(
                    v4.strokes
                        .into_iter()
                        .map(|v4| Stroke {
                            points: {
                                v4.points
                                    .iter()
                                    .map(|point| StrokeElement {
                                        x: point.x,
                                        y: point.y,
                                        pressure: point.pressure,
                                    })
                                    .collect()
                            },
                            color: v4.color,
                            brush_size: v4.brush_size,
                            erased: v4.erased,
                            ..Default::default()
                        })
                        .collect(),
                ),
                zoom: v4.zoom,
                ..Default::default()
            };

            return Ok(state);
        }

        Version(3) => {
            let v3: v3::StateV3 = v3::read(file)?;

            let state = Sketch {
                strokes: crate::map_from_vec(
                    v3.strokes
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
                ),
                zoom: v3.zoom,
                ..Default::default()
            };

            return Ok(state);
        }

        Version(2) => {
            let v2: v2::StateV2 = v2::read(file)?;

            let state = Sketch {
                strokes: crate::map_from_vec(
                    v2.strokes
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
                ),
                zoom: v2.zoom,
                ..Default::default()
            };

            return Ok(state);
        }

        Version(1) => {
            let v1: v1::StateV1 = v1::read(file)?;

            let state = Sketch {
                strokes: crate::map_from_vec(
                    v1.strokes
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
                ),
                zoom: v1.zoom,
                ..Default::default()
            };

            return Ok(state);
        }

        _ => Err(PmbError::new(ErrorKind::UnknownVersion(version))),
    }
}

mod v5 {
    use super::*;

    #[derive(bincode::Decode)]
    pub struct StrokeElementV5 {
        pub x: f32,
        pub y: f32,
        pub pressure: f32,
    }

    #[derive(bincode::Decode)]
    pub struct SketchV5 {
        pub strokes: Vec<StrokeV5>,
        pub zoom: f32,
        pub origin: StrokePointV5,
    }

    #[derive(bincode::Decode)]
    pub struct StrokePointV5 {
        pub x: f32,
        pub y: f32,
    }

    #[derive(bincode::Decode)]
    pub struct StrokeV5 {
        pub points: Vec<StrokeElementV5>,
        pub color: [u8; 3],
        pub brush_size: f32,
        pub erased: bool,
    }

    pub fn read(mut reader: impl Read) -> Result<SketchV5, PmbError> {
        let mut magic = [0; 3];
        reader.read_exact(&mut magic)?;

        if magic != crate::PMB_MAGIC {
            return Err(PmbError::new(ErrorKind::MissingHeader));
        }

        let mut version_bytes = [0; std::mem::size_of::<u64>()];
        reader.read_exact(&mut version_bytes)?;
        let version = Version(u64::from_le_bytes(version_bytes));

        log::debug!("got version {}", version);
        if version != Version(5) {
            return Err(PmbError::new(ErrorKind::VersionMismatch(version)));
        }

        log::debug!("inflating");
        let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
        Ok(bincode::decode_from_std_read(
            &mut deflate_reader,
            bincode::config::standard(),
        )?)
    }
}

mod v4 {
    use super::*;

    #[derive(bincode::Decode)]
    pub struct StrokeElementV4 {
        pub x: f32,
        pub y: f32,
        pub pressure: f32,
    }

    #[derive(bincode::Decode)]
    pub struct StrokePointV4 {
        pub x: f32,
        pub y: f32,
    }

    #[derive(bincode::Decode)]
    pub struct StrokeV4 {
        pub points: Vec<StrokeElementV4>,
        pub color: [u8; 3],
        pub brush_size: f32,
        pub erased: bool,
    }

    #[derive(bincode::Decode)]
    pub struct StateV4 {
        pub strokes: Vec<StrokeV4>,
        pub brush_size: usize,
        pub zoom: f32,
        pub origin: StrokePointV4,
    }

    pub fn read(mut reader: impl Read) -> Result<StateV4, PmbError> {
        let mut magic = [0; 3];
        reader.read_exact(&mut magic)?;

        if magic != crate::PMB_MAGIC {
            return Err(PmbError::new(ErrorKind::MissingHeader));
        }

        let mut version_bytes = [0; std::mem::size_of::<u64>()];
        reader.read_exact(&mut version_bytes)?;
        let version = Version(u64::from_le_bytes(version_bytes));

        if version != Version(4) {
            return Err(PmbError::new(ErrorKind::VersionMismatch(version)));
        }

        let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
        Ok(bincode::decode_from_std_read(
            &mut deflate_reader,
            standard(),
        )?)
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
    pub struct StrokePointV1 {
        pub x: f32,
        pub y: f32,
    }

    #[derive(bincode::Decode)]
    #[repr(C)]
    pub struct StrokeElementV1 {
        pub x: f32,
        pub y: f32,
        pub pressure: f32,
    }

    #[derive(bincode::Decode)]
    pub struct StrokeV1 {
        pub points: Vec<StrokeElementV1>,
        pub color: [u8; 3],
        pub brush_size: f32,
        pub erased: bool,
    }

    #[derive(bincode::Decode)]
    pub struct StateV1 {
        pub strokes: Vec<StrokeV1>,
        pub brush_size: usize,
        pub zoom: f32,
        pub origin: StrokePointV1,
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
