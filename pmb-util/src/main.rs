use anyhow::Result;
use gumdrop::Options;
use powdermilk_biscuits::{
    config::Config,
    migrate::{self, v1, v2, v3, v4, v5, v6, v7, v8, Version},
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
    version: bool,

    #[options(help = "Print the default config file", no_short)]
    print_default_config: bool,

    #[options(
        help = "Attempt to upgrade the file to the latest version",
        short = "M"
    )]
    migrate: bool,

    #[options(
        help = "Migrate in-place. Potentially dangerous. Requires -M/--migrate",
        no_short
    )]
    migrate_in_place: bool,

    #[options(
        help = "Do not write any changes to disk. Requires -M/--migrate and not --migrate-in-place",
        no_short
    )]
    dry_run: bool,

    #[options(free, help = "File to analyze")]
    path: Option<PathBuf>,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse_args_default_or_exit();

    if 1 < [args.version, args.print_default_config, args.migrate]
        .into_iter()
        .fold(0, |acc, b| if b { acc + 1 } else { acc })
        || (!args.migrate && args.migrate_in_place)
        || (!args.migrate && args.dry_run)
        || (args.migrate && args.migrate_in_place && args.dry_run)
    {
        println!("{}", Args::usage());
        return Err(anyhow::anyhow!("Invalid usage"));
    }

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
        let about = look_at(path)?;

        if args.migrate {
            if about.version() == Version::CURRENT {
                println!("{} already up to date", path.display());
                return Ok(());
            }

            println!("Migrating {}", path.display());
            let new = migrate::from::<()>(about.version(), path)?;

            if args.dry_run {
                println!("Successful, aborting due to --dry-run");
                return Ok(());
            }

            let write_path = if args.migrate_in_place {
                path.clone()
            } else {
                let new_name = format!(
                    "{}_v{}.pmb",
                    path.file_stem().unwrap().to_str().unwrap(),
                    Version::CURRENT,
                );
                PathBuf::from(new_name)
            };

            println!("Saving as {}", write_path.display());
            migrate::write(write_path, &new)?;
        } else {
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
                    _ => unreachable!("missing version in read! macro call")
                }
            }
        };
    }

    read!(1, 2, 3, 4, 5, 6, 7, 8)
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

    fn num_erased_strokes(&self) -> Option<usize> {
        None
    }

    fn show(&self) {
        println!("PMB file v{}: {}", self.version(), self.changes());
        println!(
            "{} strokes{}",
            self.num_strokes(),
            if let Some(num_erased) = self.num_erased_strokes() {
                format!(", {} erased", num_erased)
            } else {
                String::new()
            }
        );
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
        if let Some(fg_color) = self.fg_color() {
            println!(
                "fg color: ({:.02}, {:.02}, {:.02})",
                fg_color[0], fg_color[1], fg_color[2],
            );
        }
    }
}

impl About for Sketch<()> {
    fn changes(&self) -> &'static str {
        "Added foreground color, removed erased strokes"
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

impl About for v8::SketchV8 {
    fn changes(&self) -> &'static str {
        "Identical to v7"
    }

    fn version(&self) -> Version {
        Version(8)
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

    fn num_erased_strokes(&self) -> Option<usize> {
        Some(self.strokes.iter().filter(|stroke| stroke.erased).count())
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

    fn num_erased_strokes(&self) -> Option<usize> {
        Some(self.strokes.iter().filter(|stroke| stroke.erased).count())
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

    fn num_erased_strokes(&self) -> Option<usize> {
        Some(self.strokes.iter().filter(|stroke| stroke.erased).count())
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

    fn num_erased_strokes(&self) -> Option<usize> {
        Some(self.strokes.iter().filter(|stroke| stroke.erased).count())
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

    fn num_erased_strokes(&self) -> Option<usize> {
        Some(self.strokes.iter().filter(|stroke| stroke.erased).count())
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

    fn num_erased_strokes(&self) -> Option<usize> {
        Some(self.strokes.iter().filter(|stroke| stroke.erased).count())
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

    fn num_erased_strokes(&self) -> Option<usize> {
        Some(self.strokes.iter().filter(|stroke| stroke.erased).count())
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

    fn num_erased_strokes(&self) -> Option<usize> {
        Some(self.strokes.iter().filter(|stroke| stroke.erased).count())
    }

    fn brush_size(&self) -> Option<usize> {
        Some(self.brush_size)
    }
}
