#[path = "build/icons.rs"]
mod icons;

#[path = "build/fonts.rs"]
mod fonts;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    icons::run_icons()?;
    fonts::run_fonts()?;
    Ok(())
}
