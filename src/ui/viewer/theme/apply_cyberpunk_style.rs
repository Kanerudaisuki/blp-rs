use egui::style::{Selection, WidgetVisuals};

pub fn apply_cyberpunk_style(ctx: &egui::Context) {
    use egui::*;

    let cyan = Color32::from_rgb(0, 220, 255);
    let mag = Color32::from_rgb(255, 60, 240);
    let fg = Color32::from_rgb(210, 240, 255);
    let bg = Color32::from_rgb(6, 10, 14); // почти чёрный с сине-зелёным

    let mut style = (*ctx.style()).clone();
    let mut v = Visuals::dark();

    v.override_text_color = Some(fg);
    v.panel_fill = bg;
    v.window_fill = Color32::from_rgba_unmultiplied(14, 20, 28, 220); // лёгкая прозрачность
    v.window_stroke = Stroke::new(1.0, Color32::from_rgb(20, 120, 160));
    v.widgets.inactive = WidgetVisuals {
        bg_fill: Color32::from_rgba_unmultiplied(18, 30, 40, 180), //
        weak_bg_fill: Default::default(),
        bg_stroke: Stroke::new(1.0, Color32::from_rgb(40, 160, 200)),
        corner_radius: Default::default(),
        fg_stroke: Stroke::new(1.0, fg),
        expansion: 0.0,
    };
    v.widgets.hovered = WidgetVisuals {
        bg_fill: Color32::from_rgba_unmultiplied(30, 50, 70, 210), //
        weak_bg_fill: Default::default(),
        bg_stroke: Stroke::new(1.5, cyan),
        corner_radius: Default::default(),
        fg_stroke: Stroke::new(1.0, fg),
        expansion: 0.0,
    };
    v.widgets.active = WidgetVisuals {
        bg_fill: Color32::from_rgba_unmultiplied(35, 60, 85, 230), //
        weak_bg_fill: Default::default(),
        bg_stroke: Stroke::new(2.0, mag),
        corner_radius: Default::default(),
        fg_stroke: Stroke::new(1.0, fg),
        expansion: 0.0,
    };
    v.widgets.noninteractive = WidgetVisuals {
        bg_fill: Default::default(),
        bg_stroke: Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 220, 255, 128)), //
        weak_bg_fill: Default::default(),
        corner_radius: Default::default(),
        fg_stroke: Stroke::new(1.0, fg),
        expansion: 0.0,
    };

    v.selection = Selection { bg_fill: Color32::from_rgba_unmultiplied(0, 220, 255, 80), stroke: Stroke::new(1.0, cyan) };
    v.hyperlink_color = cyan;
    v.warn_fg_color = mag;
    v.error_fg_color = Color32::from_rgb(255, 80, 120);

    style.visuals = v;
    style.spacing.item_spacing = vec2(8.0, 6.0);
    style.spacing.button_padding = vec2(10.0, 6.0);

    ctx.set_style(style);
}
