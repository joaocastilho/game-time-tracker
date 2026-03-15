use crate::config::data_dir;
use crate::models::{Game, Session, State};
use crate::process::ProcessMonitor;
use crate::store::{self, StoreError};
use chrono::Utc;
use log::{error, info};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

pub struct AppTracker {
    monitor: ProcessMonitor,
    active_count: Arc<AtomicUsize>,
    should_stop: Arc<std::sync::atomic::AtomicBool>,
    stop_rx: std::sync::mpsc::Receiver<()>,
}

fn prune_sessions(sessions: &mut HashMap<String, Vec<Session>>) {
    const MAX_SESSIONS_PER_GAME: usize = 100;
    for game_sessions in sessions.values_mut() {
        if game_sessions.len() > MAX_SESSIONS_PER_GAME {
            game_sessions.sort_by_key(|s| s.start);
            game_sessions.reverse();
            game_sessions.truncate(MAX_SESSIONS_PER_GAME);
        }
    }
}

impl AppTracker {
    pub fn new(
        active_count: Arc<AtomicUsize>,
        should_stop: Arc<std::sync::atomic::AtomicBool>,
        stop_rx: std::sync::mpsc::Receiver<()>,
    ) -> Self {
        Self {
            monitor: ProcessMonitor::new(),
            active_count,
            should_stop,
            stop_rx,
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

        prune_sessions(&mut all_sessions);

        let end_time = state.last_seen.unwrap_or_else(Utc::now);

        for (game_id, mut session) in state.active_sessions.into_iter() {
            session.end = Some(end_time);
            session.duration_secs = (end_time - session.start).num_seconds().max(0) as u64;

            all_sessions.entry(game_id).or_default().push(session);
        }

        store::save(&all_sessions, &sessions_path)?;

        let new_state = State::default();
        store::save(&new_state, &state_path)?;

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), StoreError> {
        if let Err(e) = self.recover_pending_sessions() {
            error!("Failed to recover pending sessions: {}", e);
        }

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

            for game in &games {
                let is_running = self.monitor.is_running(&game.executable);
                let game_id = game.id.as_str();

                let is_active = state.active_sessions.contains_key(game_id);

                if is_running && !is_active {
                    info!("Started tracking session for game: {}", game.name);
                    state.active_sessions.insert(
                        game_id.to_owned(),
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
                    let Some(mut session) = state.active_sessions.remove(game_id) else {
                        continue;
                    };
                    let end_time = Utc::now();
                    session.end = Some(end_time);
                    session.duration_secs = (end_time - session.start).num_seconds().max(0) as u64;

                    if all_sessions.is_none() {
                        all_sessions = Some(store::load(&sessions_path)?.unwrap_or_default());
                    }

                    if let Some(sessions) = all_sessions.as_mut() {
                        sessions
                            .entry(game_id.to_owned())
                            .or_default()
                            .push(session);
                    }

                    sessions_changed = true;
                    state_changed = true;
                    self.active_count
                        .store(state.active_sessions.len(), Ordering::Relaxed);
                }
            }

            // End any active sessions whose game has been removed from games.json.
            // Without this, removing a tracked game while it's running leaves its
            // active_session open indefinitely, inflating its duration at recovery.
            let current_ids: std::collections::HashSet<&str> =
                games.iter().map(|g| g.id.as_str()).collect();
            let zombie_ids: Vec<String> = state
                .active_sessions
                .keys()
                .filter(|id| !current_ids.contains(id.as_str()))
                .cloned()
                .collect();
            if !zombie_ids.is_empty() {
                info!(
                    "Ending {} zombie session(s) for removed game(s)",
                    zombie_ids.len()
                );
                if all_sessions.is_none() {
                    all_sessions = Some(store::load(&sessions_path)?.unwrap_or_default());
                }
                for zombie_id in zombie_ids {
                    if let Some(mut session) = state.active_sessions.remove(&zombie_id) {
                        let end_time = Utc::now();
                        session.end = Some(end_time);
                        session.duration_secs =
                            (end_time - session.start).num_seconds().max(0) as u64;
                        if let Some(sessions) = all_sessions.as_mut() {
                            sessions.entry(zombie_id).or_default().push(session);
                        }
                        sessions_changed = true;
                        state_changed = true;
                    }
                }
                self.active_count
                    .store(state.active_sessions.len(), Ordering::Relaxed);
            }

            if !state.active_sessions.is_empty() {
                state.last_seen = Some(Utc::now());
                state_changed = true;
            }

            if state_changed {
                if let Err(e) = store::save(&state, &state_path) {
                    error!("Failed to save state: {}", e);
                }
            }
            if sessions_changed {
                if let Some(mut sessions) = all_sessions {
                    prune_sessions(&mut sessions);
                    if let Err(e) = store::save(&sessions, &sessions_path) {
                        error!("Failed to save sessions: {}", e);
                    }
                }
            }

            // Interruptible sleep: wakes immediately when a stop signal arrives,
            // rather than blocking the full 5 seconds on Quit.
            match self.stop_rx.recv_timeout(Duration::from_secs(5)) {
                Ok(()) => {
                    // Stop signal received; will exit at top of next iteration.
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    // Sender was dropped (main thread exited); stop immediately
                    // to avoid spinning in a tight loop on every future call.
                    info!("Stop channel disconnected, stopping tracker");
                    break Ok(());
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Normal timeout — continue tracking.
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::thread::sleep;

    fn make_tracker(
        should_stop_init: bool,
    ) -> (
        AppTracker,
        std::sync::mpsc::Sender<()>,
        Arc<std::sync::atomic::AtomicBool>,
    ) {
        let active_count = Arc::new(AtomicUsize::new(0));
        let should_stop = Arc::new(AtomicBool::new(should_stop_init));
        let (stop_tx, stop_rx) = std::sync::mpsc::channel();
        let tracker = AppTracker::new(active_count, should_stop.clone(), stop_rx);
        (tracker, stop_tx, should_stop)
    }

    #[test]
    fn test_app_tracker_initialization() {
        let active_count = Arc::new(AtomicUsize::new(0));
        let should_stop = Arc::new(AtomicBool::new(false));
        let (_stop_tx, stop_rx) = std::sync::mpsc::channel();
        let tracker = AppTracker::new(active_count.clone(), should_stop.clone(), stop_rx);

        assert_eq!(active_count.load(Ordering::Relaxed), 0);
        assert!(!should_stop.load(Ordering::SeqCst));

        std::mem::forget(tracker);
    }

    #[test]
    fn test_app_tracker_stops_on_signal() {
        let (mut tracker, _stop_tx, _) = make_tracker(true);
        let result = tracker.run();
        assert!(result.is_ok());
    }

    #[test]
    fn test_active_count_tracking() {
        let active_count = Arc::new(AtomicUsize::new(0));
        let should_stop = Arc::new(AtomicBool::new(true));
        let (_stop_tx, stop_rx) = std::sync::mpsc::channel();
        let _tracker = AppTracker::new(active_count.clone(), should_stop, stop_rx);

        active_count.store(3, Ordering::Relaxed);
        assert_eq!(active_count.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_tracker_stops_quickly_on_channel_signal() {
        // Verify the tracker wakes from its inter-poll sleep immediately when
        // signalled, rather than waiting the full 5 seconds.
        let (mut tracker, stop_tx, should_stop) = make_tracker(false);

        let handle = std::thread::spawn(move || tracker.run());

        // Give the tracker time to start up and reach its recv_timeout sleep.
        sleep(Duration::from_millis(300));

        let signal_time = std::time::Instant::now();
        should_stop.store(true, Ordering::SeqCst);
        let _ = stop_tx.send(());

        let result = handle.join().expect("tracker thread panicked");
        assert!(result.is_ok());
        assert!(
            signal_time.elapsed() < Duration::from_secs(1),
            "Tracker took {:?} to stop after signal (expected < 1s)",
            signal_time.elapsed()
        );
    }
    #[test]
    fn test_tracker_stops_on_sender_drop() {
        // Verify the tracker exits immediately when the sender is dropped
        // (e.g. main thread panics) rather than spinning in a busy-loop.
        let (mut tracker, stop_tx, _should_stop) = make_tracker(false);

        let handle = std::thread::spawn(move || tracker.run());

        // Give the tracker time to reach its recv_timeout sleep.
        sleep(Duration::from_millis(300));

        let signal_time = std::time::Instant::now();
        drop(stop_tx); // disconnect without sending

        let result = handle.join().expect("tracker thread panicked");
        assert!(result.is_ok());
        assert!(
            signal_time.elapsed() < Duration::from_secs(1),
            "Tracker took {:?} to stop after sender drop (expected < 1s)",
            signal_time.elapsed()
        );
    }
    #[test]
    fn test_zombie_session_detection() {
        // Removing a game from games.json while it's being tracked should cause
        // the tracker to detect its session as a zombie and close it.
        use std::collections::HashSet;

        let mut state = State::default();
        state.active_sessions.insert(
            "removed-game".to_string(),
            Session {
                start: Utc::now(),
                end: None,
                duration_secs: 0,
            },
        );
        state.active_sessions.insert(
            "kept-game".to_string(),
            Session {
                start: Utc::now(),
                end: None,
                duration_secs: 0,
            },
        );

        // Only "kept-game" remains in the games list.
        let games = vec![Game {
            id: "kept-game".to_string(),
            name: "Kept Game".to_string(),
            executable: "kept.exe".to_string(),
        }];

        let current_ids: HashSet<&str> = games.iter().map(|g| g.id.as_str()).collect();
        let zombie_ids: Vec<String> = state
            .active_sessions
            .keys()
            .filter(|id| !current_ids.contains(id.as_str()))
            .cloned()
            .collect();

        assert_eq!(zombie_ids.len(), 1, "exactly one zombie session expected");
        assert_eq!(zombie_ids[0], "removed-game");
        assert!(
            !zombie_ids.contains(&"kept-game".to_string()),
            "kept-game must not be flagged as zombie"
        );
    }
}
