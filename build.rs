use std::{io, path::PathBuf, process::Command, str};

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

fn main() -> Result<(), BuildError> {
    let manifest_path = env!("CARGO_MANIFEST_DIR");
    let mut src_shaders_path = PathBuf::from(manifest_path);
    src_shaders_path.push("src");
    src_shaders_path.push("shaders");

    for entry in src_shaders_path.read_dir()? {
        let entry = entry?;
        let path = entry.path();
        let path_as_str = path.to_str().unwrap();
        if let Some(extension) = path.extension().map(|os_str| os_str.to_str().unwrap()) {
            if matches!(extension, "vert" | "frag" | "vs" | "fs") {
                let validator_output =
                    Command::new("glslangValidator").arg(path_as_str).output()?;
                if !validator_output.status.success() {
                    let mut message = String::from("stdout:\n");
                    message.push_str(str::from_utf8(&validator_output.stdout)?);
                    if !validator_output.stderr.is_empty() {
                        message.push_str("stderr:\n");
                        message.push_str(str::from_utf8(&validator_output.stderr)?);
                    }
                    panic!("{message}");
                }
            }
        }
    }

    Ok(())
}
