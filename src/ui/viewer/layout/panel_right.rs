use crate::image_blp::MAX_MIPS;
use crate::ui::viewer::app::App;
use eframe::egui::{Button, Context, Frame, Margin, Response, RichText, ScrollArea, Sense, SidePanel, TextStyle, Ui, vec2};

impl App {
    pub(crate) fn draw_panel_right(&mut self, ctx: &Context) {
        SidePanel::right("right_mips")
            .resizable(false)
            .exact_width(180.0)
            .show_separator_line(false)
            .frame(Frame { inner_margin: Margin::same(0), ..Default::default() })
            .show(ctx, |ui| {
                let sp = ui.spacing();
                let spx_f = sp.item_spacing.x;
                let spy_f = sp.item_spacing.y;
                let spx_i = spx_f.round() as i8;

                ScrollArea::vertical()
                    .id_salt("left_scroll_mips")
                    .show(ui, |ui| {
                        Frame { inner_margin: Margin { left: spx_i, right: spx_i, top: 0, bottom: 0 }, ..Default::default() }.show(ui, |ui| {
                            ui.add_space(spy_f * 2.0);
                            ui.add_enabled_ui(!self.loading, |ui| {
                                for i in 0..MAX_MIPS {
                                    let (w, h) = self
                                        .blp
                                        .as_ref()
                                        .and_then(|b| b.mipmaps.get(i))
                                        .map(|m| (m.width, m.height))
                                        .unwrap_or((0, 0));

                                    mipmap_button_row(ui, &mut self.mip_visible[i], i, w, h);
                                }

                                // Кнопки All / None, поровну по ширине
                                let row_h = ui.spacing().interact_size.y;
                                ui.columns(2, |cols| {
                                    if cols[0]
                                        .add_sized([cols[0].available_width(), row_h], Button::new("All").wrap())
                                        .clicked()
                                    {
                                        self.mip_visible.fill(true);
                                    }

                                    if cols[1]
                                        .add_sized([cols[1].available_width(), row_h], Button::new("None").wrap())
                                        .clicked()
                                    {
                                        self.mip_visible.fill(false);
                                    }
                                });
                            });
                        });

                        let _ = ui.allocate_exact_size(vec2(ui.available_width(), 0.0), Sense::hover());
                    });
            });
    }
}

pub fn mipmap_button_row(ui: &mut Ui, on: &mut bool, i: usize, w: u32, h: u32) -> Response {
    let row_h = ui.spacing().interact_size.y;
    let width = ui.available_width();

    let label = RichText::new(format!("#{i:02}    {w}×{h}")).text_style(TextStyle::Monospace);

    let mut btn = Button::new(label)
        .min_size(vec2(width, row_h))
        .wrap();

    //if *on {btn = btn.fill(ui.visuals().selection.bg_fill);}

    let resp = ui.add(btn);

    if resp.clicked() {
        *on = !*on;
    }

    resp
}
