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
//! - Copy/paste every type with `derive(Disk)` to that module, suffixing its name with V\[new\],
//!   removing any fields that do not get serialized to disk, and moving any `#[custom_enc]` fields
//!   after any normal fields
//! - Derive bincode::Decode for each type
//! - Add the new version to [Version::upgrade_type] and edit the compatibility between the old and
//!   new [Sketch]es
//! - Add an About impl in pmb_util for the old version
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

    tracing::debug!("got version {}", version);
    if version != Version::CURRENT {
        return Err(PmbError::new(ErrorKind::VersionMismatch(version)));
    }

    tracing::debug!("inflating");
    let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
    Ok(bincode::decode_from_std_read(
        &mut deflate_reader,
        standard(),
    )?)
}

pub fn write<S: StrokeBackend>(
    path: impl AsRef<std::path::Path>,
    state: &Sketch<S>,
) -> Result<(), PmbError> {
    tracing::debug!("truncating {} and deflating", path.as_ref().display());

    let mut file = std::fs::File::create(&path)?;
    file.write_all(&crate::PMB_MAGIC)?;
    file.write_all(&u64::to_le_bytes(Version::CURRENT.0))?;

    let mut deflate_writer = flate2::write::DeflateEncoder::new(file, flate2::Compression::fast());
    bincode::encode_into_std_write(state, &mut deflate_writer, standard())?;

    Ok(())
}

#[derive(Debug, PartialEq)]
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
    pub const CURRENT: Self = Version(9);

    pub fn upgrade_type(from: Self) -> UpgradeType {
        use UpgradeType::*;

        if from == Self::CURRENT {
            return Smooth;
        }

        match from {
            Version(5..=8) => Smooth,
            Version(1..=4) => Rocky,
            _ => Incompatible,
        }
    }

    pub fn new(number: u64) -> Result<Self, PmbError> {
        if (1..=Version::CURRENT.0).contains(&number) {
            Ok(Version(number))
        } else {
            Err(PmbError::new(ErrorKind::IncompatibleVersion(Version(
                number,
            ))))
        }
    }
}

#[allow(clippy::needless_return)]
pub fn from<S>(version: Version, path: impl AsRef<Path>) -> Result<Sketch<S>, PmbError>
where
    S: StrokeBackend,
{
    tracing::info!(
        "upgrading from {} to {} is {:?}",
        version,
        Version::CURRENT,
        Version::upgrade_type(version)
    );

    if Version::upgrade_type(version) == UpgradeType::Incompatible {
        return Err(PmbError::new(ErrorKind::IncompatibleVersion(version)));
    }

    use crate::{
        graphics::{Color, ColorExt, StrokePoint},
        stroke::*,
    };

    let file = std::fs::File::open(&path)?;

    match version {
        version if version == Version::CURRENT => unreachable!(),

        Version(8) => {
            let v8: v8::SketchV8 = v8::read(file)?;

            let state = Sketch {
                strokes: crate::map_from_vec(
                    v8.strokes
                        .into_iter()
                        .filter(|v8| !v8.erased)
                        .map(|v8| Stroke {
                            points: {
                                v8.points
                                    .iter()
                                    .map(|point| StrokeElement {
                                        x: point.x,
                                        y: point.y,
                                        pressure: point.pressure,
                                    })
                                    .collect()
                            },
                            color: v8.color,
                            brush_size: v8.brush_size,
                            ..Default::default()
                        })
                        .collect(),
                ),
                zoom: v8.zoom,
                origin: StrokePoint {
                    x: v8.origin.x,
                    y: v8.origin.y,
                },
                bg_color: v8.bg_color,
                fg_color: Color::WHITE,
            };

            return Ok(state);
        }

        Version(7) => {
            let v7: v7::SketchV7 = v7::read(file)?;

            let state = Sketch {
                strokes: crate::map_from_vec(
                    v7.strokes
                        .into_iter()
                        .filter(|v7| !v7.erased)
                        .map(|v7| Stroke {
                            points: {
                                v7.points
                                    .iter()
                                    .map(|point| StrokeElement {
                                        x: point.x,
                                        y: point.y,
                                        pressure: point.pressure,
                                    })
                                    .collect()
                            },
                            color: v7.color,
                            brush_size: v7.brush_size,
                            ..Default::default()
                        })
                        .collect(),
                ),
                zoom: v7.zoom,
                origin: StrokePoint {
                    x: v7.origin.x,
                    y: v7.origin.y,
                },
                bg_color: v7.bg_color,
                fg_color: Color::WHITE,
            };

            return Ok(state);
        }

        Version(6) => {
            let v6: v6::SketchV6 = v6::read(file)?;

            let state = Sketch {
                strokes: crate::map_from_vec(
                    v6.strokes
                        .into_iter()
                        .filter(|v6| !v6.erased)
                        .map(|v6| Stroke {
                            points: {
                                v6.points
                                    .iter()
                                    .map(|point| StrokeElement {
                                        x: point.x,
                                        y: point.y,
                                        pressure: point.pressure,
                                    })
                                    .collect()
                            },
                            color: Color::from_u8(v6.color),
                            brush_size: v6.brush_size,
                            ..Default::default()
                        })
                        .collect(),
                ),
                zoom: v6.zoom,
                origin: StrokePoint {
                    x: v6.origin.x,
                    y: v6.origin.y,
                },
                bg_color: Color::BLACK,
                fg_color: Color::WHITE,
            };

            return Ok(state);
        }

        Version(5) => {
            let v5: v5::SketchV5 = v5::read(file)?;

            let state = Sketch {
                strokes: crate::map_from_vec(
                    v5.strokes
                        .into_iter()
                        .filter(|v5| !v5.erased)
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
                            color: Color::from_u8(v5.color),
                            brush_size: v5.brush_size,
                            ..Default::default()
                        })
                        .collect(),
                ),
                zoom: v5.zoom,
                origin: StrokePoint {
                    x: v5.origin.x,
                    y: v5.origin.y,
                },
                bg_color: Color::BLACK,
                fg_color: Color::WHITE,
            };

            return Ok(state);
        }

        Version(4) => {
            let v4: v4::StateV4 = v4::read(file)?;

            let state = Sketch {
                strokes: crate::map_from_vec(
                    v4.strokes
                        .into_iter()
                        .filter(|v4| !v4.erased)
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
                            color: Color::from_u8(v4.color),
                            brush_size: v4.brush_size,
                            ..Default::default()
                        })
                        .collect(),
                ),
                zoom: v4.zoom,
                origin: StrokePoint {
                    x: v4.origin.x,
                    y: v4.origin.y,
                },
                bg_color: Color::BLACK,
                fg_color: Color::WHITE,
            };

            return Ok(state);
        }

        Version(3) => {
            let v3: v3::StateV3 = v3::read(file)?;

            let state = Sketch {
                strokes: crate::map_from_vec(
                    v3.strokes
                        .into_iter()
                        .filter(|v3| !v3.erased)
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
                            color: Color::from_u8(v3.color),
                            brush_size: v3.brush_size,
                            ..Default::default()
                        })
                        .collect(),
                ),
                zoom: v3.zoom,
                origin: StrokePoint {
                    x: v3.origin.x,
                    y: v3.origin.y,
                },
                bg_color: Color::BLACK,
                fg_color: Color::WHITE,
            };

            return Ok(state);
        }

        Version(2) => {
            let v2: v2::StateV2 = v2::read(file)?;

            let state = Sketch {
                strokes: crate::map_from_vec(
                    v2.strokes
                        .into_iter()
                        .filter(|v3| !v3.erased)
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
                            color: Color::from_u8(v2.color),
                            brush_size: v2.brush_size,
                            ..Default::default()
                        })
                        .collect(),
                ),
                zoom: v2.zoom,
                origin: StrokePoint {
                    x: v2.origin.x,
                    y: v2.origin.y,
                },
                bg_color: Color::BLACK,
                fg_color: Color::WHITE,
            };

            return Ok(state);
        }

        Version(1) => {
            let v1: v1::StateV1 = v1::read(file)?;

            let state = Sketch {
                strokes: crate::map_from_vec(
                    v1.strokes
                        .into_iter()
                        .filter(|v2| !v2.erased)
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
                            color: Color::from_u8(v1.color),
                            brush_size: v1.brush_size,
                            ..Default::default()
                        })
                        .collect(),
                ),
                zoom: v1.zoom,
                origin: StrokePoint {
                    x: v1.origin.x,
                    y: v1.origin.y,
                },
                bg_color: Color::BLACK,
                fg_color: Color::WHITE,
            };

            return Ok(state);
        }

        _ => Err(PmbError::new(ErrorKind::UnknownVersion(version))),
    }
}

pub mod v8 {
    use super::*;

    #[derive(bincode::Decode)]
    pub struct StrokePointV8 {
        pub x: f32,
        pub y: f32,
    }

    #[derive(bincode::Decode)]
    pub struct StrokeElementV8 {
        pub x: f32,
        pub y: f32,
        pub pressure: f32,
    }

    #[derive(bincode::Decode)]
    pub struct StrokeV8 {
        pub points: Vec<StrokeElementV8>,
        pub color: [f32; 3],
        pub brush_size: f32,
        pub erased: bool,
    }

    #[derive(bincode::Decode)]
    pub struct SketchV8 {
        pub zoom: f32,
        pub origin: StrokePointV8,
        pub bg_color: [f32; 3],
        pub strokes: Vec<StrokeV8>,
    }

    pub fn read(mut reader: impl Read) -> Result<SketchV8, PmbError> {
        let mut magic = [0; 3];
        reader.read_exact(&mut magic)?;

        if magic != crate::PMB_MAGIC {
            return Err(PmbError::new(ErrorKind::MissingHeader));
        }

        let mut version_bytes = [0; std::mem::size_of::<u64>()];
        reader.read_exact(&mut version_bytes)?;
        let version = Version(u64::from_le_bytes(version_bytes));

        tracing::debug!("got version {}", version);
        if version != Version(8) {
            unreachable!(
                "called v8::read when you should have called v{}::read",
                version
            );
        }

        tracing::debug!("inflating");
        let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
        Ok(bincode::decode_from_std_read(
            &mut deflate_reader,
            standard(),
        )?)
    }
}

pub mod v7 {
    use super::*;

    #[derive(bincode::Decode)]
    pub struct StrokePointV7 {
        pub x: f32,
        pub y: f32,
    }

    #[derive(bincode::Decode)]
    pub struct StrokeElementV7 {
        pub x: f32,
        pub y: f32,
        pub pressure: f32,
    }

    #[derive(bincode::Decode)]
    pub struct StrokeV7 {
        pub points: Vec<StrokeElementV7>,
        pub color: [f32; 3],
        pub brush_size: f32,
        pub erased: bool,
    }

    #[derive(bincode::Decode)]
    pub struct SketchV7 {
        pub zoom: f32,
        pub origin: StrokePointV7,
        pub bg_color: [f32; 3],
        pub strokes: Vec<StrokeV7>,
    }

    pub fn read(mut reader: impl Read) -> Result<SketchV7, PmbError> {
        let mut magic = [0; 3];
        reader.read_exact(&mut magic)?;

        if magic != crate::PMB_MAGIC {
            return Err(PmbError::new(ErrorKind::MissingHeader));
        }

        let mut version_bytes = [0; std::mem::size_of::<u64>()];
        reader.read_exact(&mut version_bytes)?;
        let version = Version(u64::from_le_bytes(version_bytes));

        tracing::debug!("got version {}", version);
        if version != Version(7) {
            unreachable!(
                "called v7::read when you should have called v{}::read",
                version
            );
        }

        tracing::debug!("inflating");
        let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
        Ok(bincode::decode_from_std_read(
            &mut deflate_reader,
            standard(),
        )?)
    }
}

pub mod v6 {
    use super::*;

    #[derive(bincode::Decode)]
    pub struct StrokePointV6 {
        pub x: f32,
        pub y: f32,
    }

    #[derive(bincode::Decode)]
    pub struct StrokeElementV6 {
        pub x: f32,
        pub y: f32,
        pub pressure: f32,
    }

    #[derive(bincode::Decode)]
    pub struct StrokeV6 {
        pub points: Vec<StrokeElementV6>,
        pub color: [u8; 3],
        pub brush_size: f32,
        pub erased: bool,
    }

    #[derive(bincode::Decode)]
    pub struct SketchV6 {
        pub zoom: f32,
        pub origin: StrokePointV6,
        pub strokes: Vec<StrokeV6>,
    }

    pub fn read(mut reader: impl Read) -> Result<SketchV6, PmbError> {
        let mut magic = [0; 3];
        reader.read_exact(&mut magic)?;

        if magic != crate::PMB_MAGIC {
            return Err(PmbError::new(ErrorKind::MissingHeader));
        }

        let mut version_bytes = [0; std::mem::size_of::<u64>()];
        reader.read_exact(&mut version_bytes)?;
        let version = Version(u64::from_le_bytes(version_bytes));

        tracing::debug!("got version {}", version);
        if version != Version(6) {
            unreachable!(
                "called v6::read when you should have called v{}::read",
                version
            );
        }

        tracing::debug!("inflating");
        let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
        Ok(bincode::decode_from_std_read(
            &mut deflate_reader,
            standard(),
        )?)
    }
}

pub mod v5 {
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

        tracing::debug!("got version {}", version);
        if version != Version(5) {
            unreachable!(
                "called v5::read when you should have called v{}::read",
                version
            );
        }

        tracing::debug!("inflating");
        let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
        Ok(bincode::decode_from_std_read(
            &mut deflate_reader,
            standard(),
        )?)
    }
}

pub mod v4 {
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
            unreachable!(
                "called v4::read when you should have called v{}::read",
                version
            );
        }

        let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
        Ok(bincode::decode_from_std_read(
            &mut deflate_reader,
            standard(),
        )?)
    }
}

pub mod v3 {
    use super::*;

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
            unreachable!(
                "called v3::read when you should have called v{}::read",
                version
            );
        }

        let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
        Ok(bincode::decode_from_std_read(
            &mut deflate_reader,
            standard(),
        )?)
    }
}

pub mod v2 {
    use super::*;

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
            unreachable!(
                "called v2::read when you should have called v{}::read",
                version
            );
        }

        let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
        Ok(bincode::decode_from_std_read(
            &mut deflate_reader,
            standard(),
        )?)
    }
}

pub mod v1 {
    use super::*;

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
            unreachable!(
                "called v1::read when you should have called v{}::read",
                version
            );
        }

        let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
        Ok(bincode::decode_from_std_read(
            &mut deflate_reader,
            standard(),
        )?)
    }
}
