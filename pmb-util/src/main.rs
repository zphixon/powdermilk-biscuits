use anyhow::Result;
use gumdrop::Options;
use powdermilk_biscuits::{
    config::Config,
    migrate::{self, v1, v2, v3, v4, v5, v6, v7, Version},
    Sketch,
};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

#[derive(gumdrop::Options, Debug)]
pub struct Args {
    #[options(help = "Show this message")]
    help: bool,

    #[options(help = "Print the version", short = "V")]
    pub version: bool,

    #[options(help = "Config file location")]
    pub config: Option<PathBuf>,

    #[options(help = "Print the default config file and exit", no_short)]
    pub print_default_config: bool,

    #[options(
        help = "Attempt to upgrade the file to the latest version",
        short = "M"
    )]
    pub migrate: bool,

    #[options(free, help = "File to analyze")]
    pub path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse_args_default_or_exit();

    if args.version {
        println!(
            "PMB file util version {} (most recent format version {})",
            env!("CARGO_PKG_VERSION"),
            Version::CURRENT
        );
        return Ok(());
    }

    if args.print_default_config {
        println!("{}", Config::new().to_ron_string());
        return Ok(());
    }

    if let Some(path) = args.path.as_ref() {
        println!("Analyzing {}", path.display());

        if args.migrate {
        } else {
            let about = look_at(path)?;
            about.show();
        }

        Ok(())
    } else {
        Err(anyhow::anyhow!("Need a file to analyze"))
    }
}

pub fn look_at(path: &Path) -> Result<Box<dyn About>> {
    let mut file = std::fs::File::open(path)?;
    let mut magic = [0; 3];
    file.read_exact(&mut magic)?;

    if magic != powdermilk_biscuits::PMB_MAGIC {
        return Err(anyhow::anyhow!(
            "The file doesn't look like a PMB file to me"
        ));
    }

    let mut version_bytes = [0; std::mem::size_of::<u64>()];
    file.read_exact(&mut version_bytes)?;
    let number = u64::from_le_bytes(version_bytes);
    let version = match Version::new(number) {
        Ok(version) => version,
        err => {
            println!("The version number seems bogus");
            err?
        }
    };
    drop(file);

    let file = std::fs::File::open(path)?;
    macro_rules! read {
        ($($version:expr),* $(,)?) => {
            paste::paste! {
                match version {
                    Version::CURRENT => Ok(Box::new(migrate::read::<()>(file)?)),
                    $(Version($version) => Ok(Box::new([<v $version>]::read(file)?)),)*
                    _ => unreachable!()
                }
            }
        };
    }

    read!(1, 2, 3, 4, 5, 6, 7)
}

pub trait About {
    fn changes(&self) -> &'static str;
    fn version(&self) -> Version;
    fn num_strokes(&self) -> usize;
    fn zoom(&self) -> f32;
    fn origin(&self) -> (f32, f32);

    fn brush_size(&self) -> Option<usize> {
        None
    }

    fn bg_color(&self) -> Option<[f32; 3]> {
        None
    }

    fn fg_color(&self) -> Option<[f32; 3]> {
        None
    }

    fn show(&self) {
        println!("PMB file v{}: {}", self.version(), self.changes());
        println!("{} strokes", self.num_strokes());
        println!(
            "zoomed in {:.02} at {:.02},{:.02}",
            self.zoom(),
            self.origin().0,
            self.origin().1,
        );
        if let Some(brush_size) = self.brush_size() {
            println!("brush size: {}px", brush_size);
        }
        if let Some(bg_color) = self.bg_color() {
            println!(
                "bg color: ({:.02}, {:.02}, {:.02})",
                bg_color[0], bg_color[1], bg_color[2],
            );
        }
    }
}

impl About for Sketch<()> {
    fn changes(&self) -> &'static str {
        "Re-ordered fields"
    }

    fn version(&self) -> Version {
        Version::CURRENT
    }

    fn num_strokes(&self) -> usize {
        self.strokes.len()
    }

    fn zoom(&self) -> f32 {
        self.zoom
    }

    fn origin(&self) -> (f32, f32) {
        (self.origin.x, self.origin.y)
    }

    fn bg_color(&self) -> Option<[f32; 3]> {
        Some(self.bg_color)
    }
}

impl About for v7::SketchV7 {
    fn changes(&self) -> &'static str {
        "Added background color, used normalized floats for color"
    }

    fn version(&self) -> Version {
        Version(7)
    }

    fn num_strokes(&self) -> usize {
        self.strokes.len()
    }

    fn zoom(&self) -> f32 {
        self.zoom
    }

    fn origin(&self) -> (f32, f32) {
        (self.origin.x, self.origin.y)
    }

    fn bg_color(&self) -> Option<[f32; 3]> {
        Some(self.bg_color)
    }
}

impl About for v6::SketchV6 {
    fn changes(&self) -> &'static str {
        "Re-ordered sketch fields"
    }

    fn version(&self) -> Version {
        Version(6)
    }

    fn num_strokes(&self) -> usize {
        self.strokes.len()
    }

    fn zoom(&self) -> f32 {
        self.zoom
    }

    fn origin(&self) -> (f32, f32) {
        (self.origin.x, self.origin.y)
    }
}

impl About for v5::SketchV5 {
    fn changes(&self) -> &'static str {
        "Removed brush size from sketch"
    }

    fn version(&self) -> Version {
        Version(5)
    }

    fn num_strokes(&self) -> usize {
        self.strokes.len()
    }

    fn zoom(&self) -> f32 {
        self.zoom
    }

    fn origin(&self) -> (f32, f32) {
        (self.origin.x, self.origin.y)
    }
}

impl About for v4::StateV4 {
    fn changes(&self) -> &'static str {
        "Recombined stroke point and pressure"
    }

    fn version(&self) -> Version {
        Version(4)
    }

    fn num_strokes(&self) -> usize {
        self.strokes.len()
    }

    fn zoom(&self) -> f32 {
        self.zoom
    }

    fn origin(&self) -> (f32, f32) {
        (self.origin.x, self.origin.y)
    }

    fn brush_size(&self) -> Option<usize> {
        Some(self.brush_size)
    }
}

impl About for v3::StateV3 {
    fn changes(&self) -> &'static str {
        "Separated stroke point position and pressure"
    }

    fn version(&self) -> Version {
        Version(3)
    }

    fn num_strokes(&self) -> usize {
        self.strokes.len()
    }

    fn zoom(&self) -> f32 {
        self.zoom
    }

    fn origin(&self) -> (f32, f32) {
        (self.origin.x, self.origin.y)
    }

    fn brush_size(&self) -> Option<usize> {
        Some(self.brush_size)
    }
}

impl About for v2::StateV2 {
    fn changes(&self) -> &'static str {
        "Identical to v1"
    }

    fn version(&self) -> Version {
        Version(2)
    }

    fn num_strokes(&self) -> usize {
        self.strokes.len()
    }

    fn zoom(&self) -> f32 {
        self.zoom
    }

    fn origin(&self) -> (f32, f32) {
        (self.origin.x, self.origin.y)
    }

    fn brush_size(&self) -> Option<usize> {
        Some(self.brush_size)
    }
}

impl About for v1::StateV1 {
    fn changes(&self) -> &'static str {
        "First version with a defined format"
    }

    fn version(&self) -> Version {
        Version(1)
    }

    fn num_strokes(&self) -> usize {
        self.strokes.len()
    }

    fn zoom(&self) -> f32 {
        self.zoom
    }

    fn origin(&self) -> (f32, f32) {
        (self.origin.x, self.origin.y)
    }

    fn brush_size(&self) -> Option<usize> {
        Some(self.brush_size)
    }
}
