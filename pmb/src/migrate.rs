use crate::{
    error::{ErrorKind, PmbError},
    Backend, State, StrokeBackend,
};

pub fn from<B, S>(version: u64) -> Result<State<B, S>, PmbError>
where
    B: Backend,
    S: StrokeBackend,
{
    match version {
        version if version == crate::PMB_VERSION => unreachable!(),
        _ => Err(PmbError::new(ErrorKind::UnknownVersion(version))),
    }
}
