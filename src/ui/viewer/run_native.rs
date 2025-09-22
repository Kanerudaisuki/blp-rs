use crate::err::error::BlpError;
use crate::ui::viewer::app::App;
use eframe::egui::{ViewportBuilder, vec2};
use eframe::{NativeOptions, Renderer};
use std::path::PathBuf;

pub fn run_native(path: Option<PathBuf>) -> Result<(), BlpError> {
    eframe::run_native(
        "blp-rs",
        NativeOptions {
            persist_window: true,
            viewport: ViewportBuilder {
                title: Some("blp-rs".to_string()), //
                app_id: Some("org.warraft.blp-rs".to_string()),
                inner_size: Some(vec2(800.0, 680.0)),
                clamp_size_to_monitor_size: Some(true),
                decorations: Some(false),
                resizable: Some(true),
                ..Default::default()
            },
            renderer: Renderer::Wgpu,
            ..Default::default()
        },
        Box::new(move |cc| -> Result<Box<dyn eframe::App>, _> {
            let mut app = App::new(&cc.egui_ctx);
            if let Err(e) = app.pick_from_file(path) {
                app.error = Some(e);
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|err| BlpError::new("ui-run-native").with_arg("msg", err.to_string()))
}
