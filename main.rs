#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use blp_rs::run::run;

fn main() {
    run().expect("Run error");
}
