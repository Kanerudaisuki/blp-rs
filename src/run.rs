use crate::cli::to_blp_command::to_blp_command;
use crate::cli::to_png_command::to_png_command;
use crate::ui::viewer::run_native::run_native;
use clap::{Parser, Subcommand};
use std::error::Error;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "blp-rs", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Help {
        path: PathBuf,
    },
    /// Конвертировать в BLP
    ToBlp {
        /// Исходный файл (например PNG)
        input: PathBuf,
        /// Опциональный выходной путь. Если не указан — расширение заменяется на .blp
        output: Option<PathBuf>,
    },

    /// Конвертировать в PNG
    ToPng {
        /// Исходный файл (например BLP)
        input: PathBuf,
        /// Опциональный выходной путь. Если не указан — расширение заменяется на .png
        output: Option<PathBuf>,
    },
}

pub fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    match Cli::try_parse() {
        Ok(cli) => match cli.command {
            Command::Help { path } => {
                println!("help {}", path.display());
            }

            Command::ToBlp { input, output } => {
                to_blp_command(&input, output.as_ref())?;
            }
            Command::ToPng { input, output } => {
                to_png_command(&input, output.as_ref())?;
            }
        },

        Err(_) => {
            let path = std::env::args_os()
                .nth(1)
                .map(PathBuf::from);
            run_native(path);
        }
    }

    Ok(())
}
