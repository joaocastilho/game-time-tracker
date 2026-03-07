use eframe::egui; fn main() { let mut builder = eframe::winit::event_loop::EventLoopBuilder::new(); eframe::winit::platform::windows::EventLoopBuilderExtWindows::any_thread(&mut builder); }
