use std::{
    io,
    path::PathBuf,
    process::{Command, Output},
    str,
};

enum GlShaderKind {
    Vertex,
    Fragment,
}

impl TryFrom<&str> for GlShaderKind {
    type Error = ();
    fn try_from(s: &str) -> Result<Self, ()> {
        match s {
            "frag" | "fs" => Ok(GlShaderKind::Fragment),
            "vert" | "vs" => Ok(GlShaderKind::Vertex),
            _ => Err(()),
        }
    }
}

impl Into<&'static str> for GlShaderKind {
    fn into(self) -> &'static str {
        match self {
            GlShaderKind::Vertex => "vert",
            GlShaderKind::Fragment => "frag",
        }
    }
}

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
        if let Some(extension) = source_path
            .extension()
            .map(|os_str| os_str.to_str().unwrap())
        {
            if cfg!(feature = "gl") {
                if let Ok(kind) = GlShaderKind::try_from(extension) {
                    if cfg!(feature = "output-spirv") {
                        let binary_filename = format!(
                            "{}_{}.spv",
                            source_path.file_stem().unwrap().to_str().unwrap(),
                            <GlShaderKind as Into<&'static str>>::into(kind),
                        );
                        let mut binary_path = target_dir.clone();
                        binary_path.push(binary_filename);

                        let output = Command::new("glslangValidator")
                            .arg("-G")
                            .arg("--target-env")
                            .arg("opengl")
                            .arg(source_path_str)
                            .arg("-o")
                            .arg(binary_path.to_str().unwrap())
                            .output()?;

                        if !output.status.success() {
                            print_message(output)?;
                        }
                    } else {
                        let output = Command::new("glslangValidator")
                            .arg(source_path_str)
                            .output()?;

                        if !output.status.success() {
                            print_message(output)?;
                        }
                    }
                }
            } else if cfg!(feature = "wgpu") && extension == "wgsl" {
                let output = Command::new("naga").arg(source_path_str).output()?;
                if !output.status.success() {
                    print_message(output)?;
                }
            }
        }
    }

    Ok(())
}
