use crate::config::data_dir;
use crate::icon;
use crate::models::{Game, Session, State};
use crate::store;
use eframe::egui;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct GameManagerApp {
    is_open: Arc<AtomicBool>,
    games: Vec<Game>,
    sessions: HashMap<String, Vec<Session>>,
    state: State,

    // Add Game fields
    new_game_name: String,
    new_game_exec: String,
    add_error: Option<String>,
}

impl GameManagerApp {
    fn new(is_open: Arc<AtomicBool>, ctx_ref: egui::Context) -> Self {
        let (games, sessions, state) = Self::load_data();
        Self {
            is_open,
            games,
            sessions,
            state,
            new_game_name: String::new(),
            new_game_exec: String::new(),
            add_error: None,
        }
    }

    fn load_data() -> (Vec<Game>, HashMap<String, Vec<Session>>, State) {
        let dir = data_dir();
        let games = store::load(dir.join("games.json"))
            .unwrap_or_default()
            .unwrap_or_default();
        let sessions = store::load(dir.join("sessions.json"))
            .unwrap_or_default()
            .unwrap_or_default();
        let state = store::load(dir.join("state.json"))
            .unwrap_or_default()
            .unwrap_or_default();
        (games, sessions, state)
    }

    fn save_games(&self) {
        let path = data_dir().join("games.json");
        if let Err(e) = store::save(&self.games, path) {
            log::error!("Failed to save games: {}", e);
        }
    }

    fn reload_all(&mut self) {
        let dir = data_dir();
        self.games = store::load(dir.join("games.json"))
            .unwrap_or_default()
            .unwrap_or_default();
        self.state = store::load(dir.join("state.json"))
            .unwrap_or_default()
            .unwrap_or_default();
        self.sessions = store::load(dir.join("sessions.json"))
            .unwrap_or_default()
            .unwrap_or_default();
    }

    fn reload_games(&mut self) {
        let dir = data_dir();
        self.games = store::load(dir.join("games.json"))
            .unwrap_or_default()
            .unwrap_or_default();
    }
}

impl eframe::App for GameManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.is_open.store(false, Ordering::SeqCst);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            return;
        }

        if !self.is_open.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            return;
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        }

        self.reload_all();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Game Time Tracker - Management");
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("↻ Refresh").clicked() {
                    self.reload_all();
                }
            });

            ui.separator();
            ui.heading("Tracked Games");

            let mut to_remove = None;

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for (idx, game) in self.games.iter().enumerate() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong(&game.name);
                                if self.state.active_sessions.contains_key(&game.id) {
                                    ui.colored_label(egui::Color32::GREEN, "▶ Running");
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("🗑 Remove").clicked() {
                                            to_remove = Some(idx);
                                        }
                                    },
                                );
                            });

                            ui.label(format!("ID: {} | Executable: {}", game.id, game.executable));

                            // Total time logic
                            let mut total_secs = 0;
                            if let Some(game_sessions) = self.sessions.get(&game.id) {
                                total_secs +=
                                    game_sessions.iter().map(|s| s.duration_secs).sum::<u64>();
                            }
                            if let Some(active) = self.state.active_sessions.get(&game.id) {
                                total_secs +=
                                    (chrono::Utc::now() - active.start).num_seconds().max(0) as u64;
                            }

                            let hours = total_secs / 3600;
                            let minutes = (total_secs % 3600) / 60;
                            ui.label(format!("Total Play Time: {}h {}m", hours, minutes));
                        });
                    }
                });

            if let Some(idx) = to_remove {
                self.games.remove(idx);
                self.save_games();
                self.reload_games();
            }

            ui.separator();
            ui.heading("Add Game");

            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.new_game_name);
            });
            ui.horizontal(|ui| {
                ui.label("Executable:");
                ui.text_edit_singleline(&mut self.new_game_exec);
            });

            if let Some(err) = &self.add_error {
                ui.colored_label(egui::Color32::RED, err);
            }

            if ui.button("➕ Add Game").clicked() {
                let trimmed_name = self.new_game_name.trim().to_string();
                let trimmed_exec = self.new_game_exec.trim().to_string();

                if trimmed_name.is_empty() || trimmed_exec.is_empty() {
                    self.add_error = Some("All fields must be filled out.".to_string());
                } else {
                    let game_id = Game::generate_id(&trimmed_name);

                    if game_id.is_empty() {
                        self.add_error = Some(
                            "Game name must contain at least one alphanumeric character."
                                .to_string(),
                        );
                    } else {
                        self.reload_games();
                        if self.games.iter().any(|g| g.id == game_id) {
                            self.add_error = Some("Game already exists.".to_string());
                        } else {
                            self.games.push(Game {
                                id: game_id,
                                name: trimmed_name,
                                executable: trimmed_exec,
                            });
                            self.save_games();
                            self.reload_games();
                            self.new_game_name.clear();
                            self.new_game_exec.clear();
                            self.add_error = None;
                        }
                    }
                }
            }
        });

        ctx.request_repaint_after_secs(1.0); // Repaint every second to update "Running" duration
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.is_open.store(false, Ordering::SeqCst);
    }
}

pub fn init_ui_thread(is_open: Arc<AtomicBool>, ctx_sender: std::sync::mpsc::Sender<egui::Context>) {
    let icon_rgba = icon::icon_rgba();
    let icon = egui::IconData {
        rgba: icon_rgba,
        width: 32,
        height: 32,
    };

    std::thread::spawn(move || {
        let mut options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([500.0, 600.0])
                .with_min_inner_size([400.0, 400.0])
                .with_visible(false)
                .with_icon(icon.clone()),
            ..Default::default()
        };

        #[cfg(target_os = "windows")]
        {
            options.event_loop_builder = Some(Box::new(|builder| {
                use winit::platform::windows::EventLoopBuilderExtWindows;
                builder.with_any_thread(true);
            }));
        }

        let result = eframe::run_native(
            "Game Time Tracker",
            options,
            Box::new(move |cc| {
                let _ = ctx_sender.send(cc.egui_ctx.clone());
                Ok(Box::new(GameManagerApp::new(is_open, cc.egui_ctx.clone())))
            }),
        );

        if let Err(e) = result {
            log::error!("eframe window error: {}", e);
        }
    });
}
