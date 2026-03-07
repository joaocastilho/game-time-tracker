use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub executable: String,
}

impl Game {
    pub fn generate_id(name: &str) -> String {
        name.trim()
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub duration_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    pub active_sessions: HashMap<String, Session>,
    pub last_seen: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_serialization() {
        let game = Game {
            id: "minecraft".to_string(),
            name: "Minecraft".to_string(),
            executable: "javaw.exe".to_string(),
        };

        let json = serde_json::to_string(&game).unwrap();
        let loaded: Game = serde_json::from_str(&json).unwrap();

        assert_eq!(game.id, loaded.id);
        assert_eq!(game.name, loaded.name);
        assert_eq!(game.executable, loaded.executable);
    }

    #[test]
    fn test_session_serialization() {
        let start = Utc::now();
        let session = Session {
            start,
            end: Some(Utc::now()),
            duration_secs: 3600,
        };

        let json = serde_json::to_string(&session).unwrap();
        let loaded: Session = serde_json::from_str(&json).unwrap();

        assert_eq!(session.duration_secs, loaded.duration_secs);
    }

    #[test]
    fn test_state_default() {
        let state = State::default();
        assert!(state.active_sessions.is_empty());
        assert!(state.last_seen.is_none());
    }

    #[test]
    fn test_state_serialization() {
        let mut state = State::default();
        state.active_sessions.insert(
            "game1".to_string(),
            Session {
                start: Utc::now(),
                end: None,
                duration_secs: 0,
            },
        );

        let json = serde_json::to_string(&state).unwrap();
        let loaded: State = serde_json::from_str(&json).unwrap();

        assert_eq!(state.active_sessions.len(), loaded.active_sessions.len());
    }

    #[test]
    fn test_generate_id_normal() {
        let id = Game::generate_id("Minecraft");
        assert_eq!(id, "minecraft");
    }

    #[test]
    fn test_generate_id_with_spaces() {
        let id = Game::generate_id("Grand Theft Auto");
        assert_eq!(id, "grand-theft-auto");
    }

    #[test]
    fn test_generate_id_with_special_chars() {
        let id = Game::generate_id("Game! @#$%");
        assert_eq!(id, "game------");
    }

    #[test]
    fn test_generate_id_all_non_alphanumeric() {
        let id = Game::generate_id("!@#$%");
        assert_eq!(id, "-----");
    }

    #[test]
    fn test_generate_id_mixed() {
        let id = Game::generate_id("Elden Ring (2022)");
        assert_eq!(id, "elden-ring--2022-");
    }
}
