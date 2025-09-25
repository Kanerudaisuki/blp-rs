#[path = "build/icons.rs"]
mod icons;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    icons::run_icons()?;
    Ok(())
}
