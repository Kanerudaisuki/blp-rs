pub(crate) use crate::ui::viewer::app::App;
#[allow(unused_imports)]
use crate::ui::viewer::layout::resize_corner_br::resize_corner_br;
use crate::ui::viewer::theme::apply_cyberpunk_style::apply_cyberpunk_style;
use crate::ui::viewer::theme::paint_bg_neon_maze::paint_bg_neon_maze;
use eframe::egui::{self};

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_os = "macos")]
        {
            use crate::ui::viewer::file_picker::macos_paste_event::{take_cmdv_event, tick_ensure_cmdv_event};
            tick_ensure_cmdv_event();
            if take_cmdv_event() {
                if let Err(e) = self.pick_from_clipboard() {
                    self.err_set(e);
                }
            }
        }

        apply_cyberpunk_style(ctx);
        paint_bg_neon_maze(ctx, self.bg_seed);
        self.draw_title_bar(ctx);
        self.draw_footer(ctx);
        self.file_picker_draw(ctx);
        if self.blp.is_some() || self.loading {
            self.draw_panel_left(ctx);
            self.draw_panel_right(ctx);
            self.draw_panel_center(ctx);
        }
        self.poll_decoder(ctx);

        #[cfg(not(target_os = "macos"))]
        resize_corner_br(ctx);
    }
}
