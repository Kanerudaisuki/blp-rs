use crate::error::error::BlpError;

#[cfg(any(feature = "cli", feature = "ui"))]
use std::path::PathBuf;

#[cfg(feature = "cli")]
use {
    crate::cli::command::to_blp::to_blp,
    crate::cli::command::to_png::to_png,
    clap::{Parser, Subcommand, error::ErrorKind},
};

#[cfg(feature = "ui")]
use crate::ui::viewer::run_native::run_native;

// ===== enforce: 'ui' всегда вместе с 'cli' =====
#[cfg(all(feature = "ui", not(feature = "cli")))]
compile_error!("Feature 'ui' requires 'cli'. Use either `--features \"cli\"` or `--features \"ui cli\"`. ");

// ======================= Команды CLI =======================

#[cfg(feature = "cli")]
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

// ======================= Разные CLI-структуры под сборки =======================

// (1) Только CLI: НЕТ PATH, подкоманда обязательна → без неё ошибка (exit 2)
#[cfg(all(feature = "cli", not(feature = "ui")))]
#[derive(Debug, Parser)]
#[command(
    name = "blp",
    version,
    about = "BLP ↔ PNG converter",
    long_about = "blp is a command-line utility for converting Warcraft III textures between BLP and PNG formats.",
    override_usage = "blp <COMMAND>",
    subcommand_required = true,     // требуем команду
    // arg_required_else_help не включаем, чтобы при пустом вводе был именно error (код 2), а не help с кодом 0
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

// (2) UI + CLI: ЛИБО PATH (для GUI), ЛИБО <COMMAND>
#[cfg(all(feature = "cli", feature = "ui"))]
#[derive(Debug, Parser)]
#[command(name = "blp", version, about = "BLP ↔ PNG converter and simple viewer for Warcraft III textures", long_about = "blp is a command-line utility for converting Warcraft III textures between BLP and PNG formats. It can also launch a native GUI viewer.", override_usage = "blp [PATH]\nblp <COMMAND>")]
struct Cli {
    /// Open the native GUI viewer with this file (used by “Open With…”).
    /// If a subcommand is provided, this argument is ignored.
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

// ======================= Хелперы =======================

#[cfg(feature = "cli")]
fn run_cli_command(cmd: Command) -> Result<(), BlpError> {
    match cmd {
        Command::ToBlp { input, output } => to_blp(&input, output.as_ref()),
        Command::ToPng { input, output } => to_png(&input, output.as_ref()),
    }
}

/// try_parse с единым поведением:
/// - Help/Version → печать и None (код 0)
/// - Прочие ошибки → печать и немедленный exit (обычно код 2)
#[cfg(feature = "cli")]
fn parse_cli_or_exit() -> Option<Cli> {
    match Cli::try_parse() {
        Ok(cli) => Some(cli),
        Err(e) => {
            match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    let _ = e.print(); // graceful 0
                    None
                }
                _ => {
                    let _ = e.print();
                    std::process::exit(e.exit_code()); // обычно 2
                }
            }
        }
    }
}

// ======================= Точки входа =======================

// UI + CLI: команда → CLI; иначе → UI (с PATH или пустой)
#[cfg(all(feature = "cli", feature = "ui"))]
pub fn run() -> Result<(), BlpError> {
    let Some(cli) = parse_cli_or_exit() else {
        return Ok(());
    };
    if let Some(cmd) = cli.command { run_cli_command(cmd) } else { run_native(cli.path) }
}

// Только CLI: команда обязательна (гарантируется атрибутом subcommand_required)
#[cfg(all(feature = "cli", not(feature = "ui")))]
pub fn run() -> Result<(), BlpError> {
    let Some(cli) = parse_cli_or_exit() else {
        return Ok(());
    };
    // здесь команда уже точно есть благодаря subcommand_required=true
    run_cli_command(cli.command)
}
