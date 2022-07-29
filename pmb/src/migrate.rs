//! PMB versions
//!
//! Perhaps there are some architectural changes that should take place to make this not so silly,
//! but I wanted to be able to do `bincode::encode()/decode()` directly on `State` and friends, so
//! now you (most likely future me) have (has) to live with this for the moment. Anyway. For the
//! most part, changes can be made to types which are written to disk as long as they have a
//! `#[disk_skip]` attribute. If you ever want to make a change involving a field without
//! `#[disk_skip]`, you need to do these things:
//!
//! - Increment Version::CURRENT
//! - Add a new module named v[old] where [old] is the previous version
//! - Copy/paste every type with `derive(Disk)` to that module, suffixing its name with V[new], and
//!   removing any fields that do not get serialized to disk
//! - Derive bincode::Decode for each type
//! - Add the new version to `Version::upgrade_type` and edit the compatibility between the old and
//!   new `State`s
//! - Add the new version to `from`, replacing the old types from the new version to the previous
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
            Version(version) if version > 5000 => Smooth,

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
    use crate::stroke::*;
    let file = std::fs::File::open(&path)?;

    match version {
        version if version == Version::CURRENT => unreachable!(),

        Version(version) if version > 5000 => match v0header::read(&file) {
            Ok(v0) => {
                let mut state = State {
                    strokes: v0
                        .strokes
                        .into_iter()
                        .map(|v0| Stroke {
                            points: v0
                                .points
                                .into_iter()
                                .map(|v0| {
                                    let x = v0.x;
                                    let y = v0.y;
                                    let pressure = v0.pressure;
                                    StrokeElement { x, y, pressure }
                                })
                                .collect(),
                            color: v0.color,
                            brush_size: v0.brush_size,
                            erased: v0.erased,
                            ..Default::default()
                        })
                        .collect(),
                    brush_size: v0.settings.brush_size,
                    zoom: v0.settings.zoom,
                    origin: v0.settings.origin,
                    ..Default::default()
                };

                state.strokes.iter_mut().for_each(Stroke::calculate_spline);
                Ok(state)
            }

            Err(err) if matches!(err.kind, ErrorKind::MissingHeader) => {
                let v0 = v0no_header::read(file)?;

                let mut state = State {
                    strokes: v0
                        .strokes
                        .into_iter()
                        .map(|v0| Stroke {
                            points: v0
                                .points
                                .into_iter()
                                .map(|v0| {
                                    let x = v0.x;
                                    let y = v0.y;
                                    let pressure = v0.pressure;
                                    StrokeElement { x, y, pressure }
                                })
                                .collect(),
                            color: v0.color,
                            brush_size: v0.brush_size,
                            erased: v0.erased,
                            ..Default::default()
                        })
                        .collect(),
                    brush_size: v0.settings.brush_size,
                    zoom: v0.settings.zoom,
                    origin: v0.settings.origin,
                    ..Default::default()
                };

                state.strokes.iter_mut().for_each(Stroke::calculate_spline);
                Ok(state)
            }

            Err(err) => return Err(err),
        },

        Version(1) => {
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

mod v0no_header {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, bincode::Decode)]
    #[repr(usize)]
    pub enum StrokeStyle {
        Lines,
        Circles,
        CirclesPressure,
        Points,
        Spline,
    }

    #[derive(bincode::Decode)]
    pub struct ToDisk {
        pub strokes: Vec<Stroke>,
        pub settings: Settings,
    }

    #[derive(bincode::Decode)]
    pub struct Settings {
        pub brush_size: usize,
        pub stroke_style: StrokeStyle,
        pub use_individual_style: bool,
        pub zoom: f32,
        pub origin: crate::StrokePoint,
    }

    #[derive(bincode::Decode)]
    pub struct Stroke {
        pub points: Vec<crate::stroke::StrokeElement>,
        pub color: crate::graphics::Color,
        pub brush_size: f32,
        pub style: StrokeStyle,
        pub erased: bool,
    }

    pub fn read(r: impl Read) -> Result<ToDisk, PmbError> {
        let mut reader = flate2::read::DeflateDecoder::new(r);
        Ok(bincode::decode_from_std_read(
            &mut reader,
            bincode::config::legacy(),
        )?)
    }
}

mod v0header {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, bincode::Decode)]
    #[repr(usize)]
    pub enum StrokeStyle {
        Lines,
        Circles,
        CirclesPressure,
        Points,
        Spline,
    }

    #[derive(bincode::Decode)]
    pub struct ToDisk {
        pub strokes: Vec<Stroke>,
        pub settings: Settings,
    }

    #[derive(bincode::Decode)]
    pub struct Settings {
        pub brush_size: usize,
        pub stroke_style: StrokeStyle,
        pub use_individual_style: bool,
        pub zoom: f32,
        pub origin: crate::StrokePoint,
    }

    #[derive(bincode::Decode)]
    pub struct Stroke {
        pub points: Vec<crate::stroke::StrokeElement>,
        pub color: crate::graphics::Color,
        pub brush_size: f32,
        pub style: StrokeStyle,
        pub erased: bool,
    }

    pub fn read(mut r: impl Read) -> Result<ToDisk, PmbError> {
        let mut magic = [0; 3];
        r.read_exact(&mut magic)?;

        if magic != [b'P', b'M', b'B'] {
            return Err(PmbError::new(ErrorKind::MissingHeader));
        }

        let mut reader = flate2::read::DeflateDecoder::new(r);
        Ok(bincode::decode_from_std_read(
            &mut reader,
            bincode::config::legacy(),
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
