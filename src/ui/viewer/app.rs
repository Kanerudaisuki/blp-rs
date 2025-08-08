pub struct App {
    pub(crate) maximized: bool,
}

impl App {
    pub fn new(ctx: &egui::Context) -> Self {
        let app = Self { maximized: false };
        app.install_fonts_and_style(ctx);
        app.apply_global_style(ctx);
        app
    }
}
