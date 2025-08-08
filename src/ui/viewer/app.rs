use crate::ui::viewer::fonts::install_fonts::install_fonts;

pub struct App {
    pub(crate) maximized: bool,
}

impl App {
    pub fn new(ctx: &egui::Context) -> Self {
        let app = Self { maximized: false };
        install_fonts(ctx);
        app
    }
}
