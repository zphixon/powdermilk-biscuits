use gumdrop::Options;
use powdermilk_biscuits::{config::Config, migrate::Version};
use std::path::PathBuf;

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

    #[options(free, help = "File to analyze")]
    pub file: Option<PathBuf>,
}

fn main() {
    let args = powdermilk_biscuits::Args::parse_args_default_or_exit();

    if args.version {
        println!(
            "PMB file util version {} (most recent format version {})",
            env!("CARGO_PKG_VERSION"),
            Version::CURRENT
        );
        return;
    }

    if args.print_default_config {
        println!("{}", Config::new().to_ron_string());
        return;
    }
}
