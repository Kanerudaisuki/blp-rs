use crate::ui::viewer::app::App;
use eframe::Renderer;
use egui::ViewportBuilder;
use std::path::PathBuf;

// ===== Настройки окна (load/save) — локальный модуль =====
mod prefs {
    use serde::{Deserialize, Serialize};
    use std::{fs, path::PathBuf};

    #[derive(Serialize, Deserialize, Default, Debug, Clone, Copy)]
    pub struct WindowSize {
        pub width: f32,
        pub height: f32,
    }

    fn prefs_path() -> Option<PathBuf> {
        // org="blp", app="blp-rs" — поменяй при необходимости
        let dirs = directories::ProjectDirs::from("com", "blp", "blp-rs")?;
        Some(dirs.config_dir().join("window.json"))
    }

    pub fn load_window_size() -> Option<[f32; 2]> {
        let path = prefs_path()?;
        let bytes = fs::read(path).ok()?;
        let ws: WindowSize = serde_json::from_slice(&bytes).ok()?;
        if ws.width.is_finite() && ws.height.is_finite() && ws.width > 0.0 && ws.height > 0.0 {
            Some([ws.width, ws.height])
        } else {
            None
        }
    }

    pub fn save_window_size(size: [f32; 2]) {
        if !(size[0].is_finite() && size[1].is_finite() && size[0] > 0.0 && size[1] > 0.0) {
            return;
        }
        if let Some(path) = prefs_path() {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let ws = WindowSize { width: size[0], height: size[1] };
            if let Ok(json) = serde_json::to_vec_pretty(&ws) {
                let _ = fs::write(path, json);
            }
        }
    }
}

// ===== Обёртка, которая сохраняет размер при выходе =====
struct AppWithWindowPersist {
    inner: App,
    last_size: [f32; 2],
}

impl AppWithWindowPersist {
    fn new(inner: App) -> Self {
        Self { inner, last_size: [0.0, 0.0] }
    }
}

impl eframe::App for AppWithWindowPersist {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // проксируем обновление в реальное приложение
        self.inner.update(ctx, frame);

        // запоминаем актуальный inner-size в каждом кадре (без рамок/декора)
        let size = ctx.input(|i| {
            i.viewport()
                .inner_rect
                .expect("REASON")
                .size()
        });
        self.last_size = [size.x, size.y];
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // сохраняем при выходе
        prefs::save_window_size(self.last_size);
        // если у твоего App есть свой on_exit — позови его тут (опционально)
        // self.inner.on_exit(_gl);
    }
}

// ===== Точка входа =====
pub fn run_native(path: Option<PathBuf>) {
    const DEFAULT_SIZE: [f32; 2] = [800.0, 684.0];
    // пытаемся восстановить прошлый размер
    let initial_size = prefs::load_window_size().unwrap_or(DEFAULT_SIZE);

    eframe::run_native(
        "blp-rs",
        eframe::NativeOptions {
            viewport: ViewportBuilder::default()
                .with_title("blp-rs")
                .with_inner_size(initial_size)
                .with_min_inner_size([480.0, 320.0]) // чтобы не схлопывалось в труху
                .with_decorations(false), // 👈 без рамок и заголовка
            // .with_transparent(true)
            // .with_always_on_top(true)
            renderer: Renderer::Wgpu, // Metal на macOS, Vulkan/DX/GL — где доступно
            ..Default::default()
        },
        Box::new(move |cc| -> Result<Box<dyn eframe::App>, _> {
            // создаём твой App как раньше
            let mut app = App::new(&cc.egui_ctx);
            app.set_initial_file(path);

            // заворачиваем в обёртку с автосейвом размера
            Ok(Box::new(AppWithWindowPersist::new(app)))
        }),
    )
    .expect("failed to run eframe");
}
