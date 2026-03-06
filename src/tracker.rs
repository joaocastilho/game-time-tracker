use crate::config::data_dir;
use crate::models::{Game, Session, State};
use crate::process::ProcessMonitor;
use crate::store::{self, StoreError};
use chrono::Utc;
use log::info;
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::thread::sleep;
use std::time::Duration;

pub struct AppTracker {
    monitor: ProcessMonitor,
    active_count: Arc<AtomicUsize>,
    should_stop: Arc<std::sync::atomic::AtomicBool>,
}

impl AppTracker {
    pub fn new(
        active_count: Arc<AtomicUsize>,
        should_stop: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            monitor: ProcessMonitor::new(),
            active_count,
            should_stop,
        }
    }

    pub fn recover_pending_sessions(&self) -> Result<(), StoreError> {
        let state_path = data_dir().join("state.json");
        let sessions_path = data_dir().join("sessions.json");

        let state: State = store::load(&state_path)?.unwrap_or_default();

        if state.active_sessions.is_empty() {
            return Ok(());
        }

        info!(
            "Recovering {} pending sessions...",
            state.active_sessions.len()
        );

        let mut all_sessions: HashMap<String, Vec<Session>> =
            store::load(&sessions_path)?.unwrap_or_default();

        let end_time = state.last_seen.unwrap_or_else(Utc::now);

        for (game_id, mut session) in state.active_sessions.into_iter() {
            session.end = Some(end_time);
            session.duration_secs = (end_time - session.start).num_seconds().max(0) as u64;

            all_sessions.entry(game_id).or_default().push(session);
        }

        // Save recovered sessions
        store::save(&all_sessions, &sessions_path)?;

        // Clear state
        let new_state = State::default();
        store::save(&new_state, &state_path)?;

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), StoreError> {
        self.recover_pending_sessions()?;

        let data_dir = data_dir();
        let games_path = data_dir.join("games.json");
        let state_path = data_dir.join("state.json");
        let sessions_path = data_dir.join("sessions.json");

        let mut state: State = store::load(&state_path)?.unwrap_or_default();

        loop {
            if self.should_stop.load(Ordering::SeqCst) {
                info!("Shutdown signal received, stopping tracker");
                break Ok(());
            }

            let games: Vec<Game> = store::load(&games_path)?.unwrap_or_default();

            let mut state_changed = false;
            let mut sessions_changed = false;
            let mut all_sessions: Option<HashMap<String, Vec<Session>>> = None;

            for game in games {
                let is_running = self.monitor.is_running(&game.executable);
                let game_id = game.id;

                let is_active = state.active_sessions.contains_key(&game_id);

                if is_running && !is_active {
                    info!("Started tracking session for game: {}", game.name);
                    state.active_sessions.insert(
                        game_id.clone(),
                        Session {
                            start: Utc::now(),
                            end: None,
                            duration_secs: 0,
                        },
                    );
                    state_changed = true;
                    self.active_count
                        .store(state.active_sessions.len(), Ordering::Relaxed);
                } else if !is_running && is_active {
                    info!("Ended session for game: {}", game.name);
                    let Some(mut session) = state.active_sessions.remove(&game_id) else {
                        continue;
                    };
                    let end_time = Utc::now();
                    session.end = Some(end_time);
                    session.duration_secs = (end_time - session.start).num_seconds().max(0) as u64;

                    if all_sessions.is_none() {
                        all_sessions = Some(store::load(&sessions_path)?.unwrap_or_default());
                    }

                    if let Some(sessions) = all_sessions.as_mut() {
                        sessions.entry(game_id).or_default().push(session);
                    }

                    sessions_changed = true;
                    state_changed = true;
                    self.active_count
                        .store(state.active_sessions.len(), Ordering::Relaxed);
                }
            }

            if !state.active_sessions.is_empty() {
                state.last_seen = Some(Utc::now());
                state_changed = true;
            }

            if state_changed {
                store::save(&state, &state_path)?;
            }
            if sessions_changed && let Some(sessions) = all_sessions {
                store::save(&sessions, &sessions_path)?;
            }

            sleep(Duration::from_secs(5));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_tracker_initialization() {
        let active_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let should_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let _tracker = AppTracker::new(active_count, should_stop);
        assert!(true);
    }
}
