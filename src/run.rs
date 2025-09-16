use crate::cli::command::to_blp::to_blp;
use crate::cli::command::to_png::to_png;
use crate::err::error::BlpError;
use crate::ui::viewer::run_native::run_native;
use clap::{Parser, Subcommand, error::ErrorKind};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "blp-rs",
    version,
    about = "BLP ↔ PNG converter and simple viewer for Warcraft III textures",
    long_about = "blp-rs is a command-line utility for converting Warcraft III textures \
                  between BLP and PNG formats. It can also launch a native GUI viewer."
)]
struct Cli {
    /// Open the native GUI viewer with this file (used by “Open With…”)
    ///
    /// If a subcommand is provided, this argument is ignored.
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Convert an image into BLP format
    ToBlp {
        /// Input file (e.g. PNG)
        input: PathBuf,
        /// Optional output path. If not specified, the extension will be replaced with .blp
        output: Option<PathBuf>,
    },

    /// Convert a BLP texture into PNG format
    ToPng {
        /// Input file (e.g. BLP)
        input: PathBuf,
        /// Optional output path. If not specified, the extension will be replaced with .png
        output: Option<PathBuf>,
    },
}

pub fn run() -> Result<(), BlpError> {
    match Cli::try_parse() {
        Ok(cli) => {
            if let Some(cmd) = cli.command {
                match cmd {
                    Command::ToBlp { input, output } => {
                        to_blp(&input, output.as_ref())?;
                    }
                    Command::ToPng { input, output } => {
                        to_png(&input, output.as_ref())?;
                    }
                }
            } else {
                // Нет подкоманды → GUI режим. path приходит из “Open With…”
                run_native(cli.path);
            }
            Ok(())
        }
        Err(e) => match e.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                e.print()?;
                Ok(())
            }
            _ => {
                e.print()?;
                std::process::exit(e.exit_code());
            }
        },
    }
}
