#[path = "build/icons.rs"]
mod icons;

#[path = "build/fonts.rs"]
mod fonts;

fn main() {
    icons::run_icons();
    fonts::run_fonts();
}
