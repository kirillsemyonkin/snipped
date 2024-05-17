pub mod paste;
pub mod scan;

use std::env::args;
use std::error::Error;
use std::io;
use std::io::stdin;
use std::io::stdout;
use std::io::Write;
use std::process::exit;

use constcat::concat;
use log::error;
use log::warn;
use log::LevelFilter::Info;
use log::LevelFilter::Off;
use simplelog::ColorChoice;
use simplelog::ConfigBuilder;
use simplelog::TerminalMode;

const PACKAGE: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const USER_AGENT: &str = concat!(PACKAGE, "/", VERSION);

fn main() -> Result<(), Box<dyn Error>> {
    simplelog::TermLogger::init(
        Info,
        ConfigBuilder::new()
            .set_time_level(Off)
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    ctrlc::set_handler(|| {
        eprintln!();
        error!("Cancelled");

        #[cfg(any(target_os = "linux", target_os = "macos"))] // idk if macos uses 130
        exit(130);
        #[cfg(target_os = "windows")]
        exit(0xC000013Au32 as i32);

        #[allow(unreachable_code)]
        {
            exit(1);
        }
    })?;

    if let Err(e) = main_main() {
        error!("{e}");
    }
    Ok(())
}

fn main_main() -> Result<(), Box<dyn Error>> {
    let mut args = args().skip(1);
    let mut first_arg = args.next();
    loop {
        let subcommand = match first_arg.clone() {
            Some(x) => x,
            None => ask("Enter subcommand (`paste`/`p`, `scan`/`s`)")?,
        };
        return match &*subcommand {
            "paste" | "p" | "v" => paste::main(args),
            "scan" | "s" => scan::main(args),
            _ => {
                warn!("Unknown subcommand: {subcommand}");
                first_arg = None;
                continue;
            },
        };
    }
}

pub(crate) fn ask(question: &str) -> io::Result<String> {
    print!("{}: ", question);
    stdout().flush()?;

    let mut input = String::new();
    stdin().read_line(&mut input)?;
    Ok(input
        .trim()
        .to_string())
}
