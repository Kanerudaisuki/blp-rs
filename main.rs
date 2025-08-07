#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use blp_rs::ui::run::run;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run().map_err(|e| {
        eprintln!("💥 Failed to run: {}", e);
        e
    })
}
