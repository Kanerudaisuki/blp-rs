use std::path::PathBuf;

use winit::event_loop::EventLoop;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

pub fn run_ui(_path: Option<PathBuf>) {
    let mut app = App::new();
    let event_loop = EventLoop::new().unwrap(); // распаковываем Result
    event_loop
        .run_app(&mut app)
        .expect("Can't run event loop");
}

pub struct App {
    window: Option<Window>,
}

impl App {
    pub fn new() -> Self {
        Self { window: None }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(WindowAttributes::default().with_title("blp-rs"))
            .expect("Failed to create window");

        let _ = window.request_inner_size(PhysicalSize::new(800, 600));
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Вставляй свою отрисовку здесь
                if let Some(window) = &self.window {
                    window.request_redraw(); // повторный redraw
                }
            }
            _ => {}
        }
    }
}
