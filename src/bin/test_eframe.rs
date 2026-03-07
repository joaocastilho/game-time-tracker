use eframe::egui;

struct App;

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

fn main() {
    println!("Before run_native");
    let options = eframe::NativeOptions::default();
    let result = eframe::run_native(
        "Test",
        options,
        Box::new(|_cc| Ok(Box::new(App))),
    );
    println!("After run_native: {:?}", result);
}
