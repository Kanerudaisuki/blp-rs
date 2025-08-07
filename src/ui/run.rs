use crate::ui::viewer::run::run_ui;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "blp-rs", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Info { path: PathBuf },

    Convert { path: PathBuf },
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::try_parse() {
        Ok(cli) => match cli.command {
            Command::Info { path } => {
                println!("info {}", path.display());
            }

            Command::Convert { path } => {
                println!("convert {}", path.display());
            }
        },

        Err(_) => {
            println!("run");
            let path = std::env::args_os()
                .nth(1)
                .map(PathBuf::from);
            run_ui(path);
        }
    }

    Ok(())
}
