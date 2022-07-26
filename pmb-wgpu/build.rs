use std::{
    io,
    path::PathBuf,
    process::{Command, Output},
    str,
};

#[derive(Debug)]
enum BuildError {
    Io(io::Error),
    Utf8(str::Utf8Error),
}

impl From<io::Error> for BuildError {
    fn from(e: io::Error) -> Self {
        BuildError::Io(e)
    }
}

impl From<str::Utf8Error> for BuildError {
    fn from(e: str::Utf8Error) -> Self {
        BuildError::Utf8(e)
    }
}

fn print_message(output: Output) -> Result<(), BuildError> {
    let mut message = String::new();
    if !output.stdout.is_empty() {
        message.push_str("stdout:\n");
        message.push_str(str::from_utf8(&output.stdout)?);
    }
    if !output.stderr.is_empty() {
        message.push_str("stderr:\n");
        message.push_str(str::from_utf8(&output.stderr)?);
    }
    panic!("{message}");
}

fn main() -> Result<(), BuildError> {
    let manifest_path = env!("CARGO_MANIFEST_DIR");

    let mut target_dir = PathBuf::from(manifest_path);
    target_dir.push("target");

    let mut src_shaders_path = PathBuf::from(manifest_path);
    src_shaders_path.push("src");
    src_shaders_path.push("shaders");

    for entry in src_shaders_path.read_dir()? {
        let entry = entry?;
        let source_path = entry.path();
        let source_path_str = source_path.to_str().unwrap();
        let output = Command::new("naga").arg(source_path_str).output()?;
        if !output.status.success() {
            print_message(output)?;
        }
    }

    Ok(())
}
