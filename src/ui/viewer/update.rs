pub(crate) use crate::ui::viewer::app::App;
#[allow(unused_imports)]
use crate::ui::viewer::resize_corner_br::resize_corner_br;
use crate::ui::viewer::theme::apply_cyberpunk_style::apply_cyberpunk_style;
use crate::ui::viewer::theme::paint_bg_neon_maze::paint_bg_neon_maze;
use eframe::egui::{self};

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_cyberpunk_style(ctx);
        paint_bg_neon_maze(ctx, self.bg_seed);
        self.draw_title_bar(ctx);
        self.draw_file_picker_bar(ctx);
        if self.picked_file.is_some() {
            self.draw_panel_right(ctx);
            self.draw_panel_center(ctx);
        }
        self.poll_decoder(ctx);

        #[cfg(not(target_os = "macos"))]
        resize_corner_br(ctx);
    }
}
