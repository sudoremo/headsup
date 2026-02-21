mod lock;
mod types;

pub use lock::FileLock;
pub use types::*;

use crate::config;
use crate::error::Result;
use std::fs;
use std::time::Duration;

/// Default lock timeout in seconds
const LOCK_TIMEOUT_SECS: u64 = 5;

/// Load state from file (with locking)
pub fn load_state() -> Result<(State, FileLock)> {
    let path = config::state_path()?;
    let lock = FileLock::acquire(&path, Duration::from_secs(LOCK_TIMEOUT_SECS))?;

    let state = if path.exists() {
        let content = fs::read_to_string(&path)?;
        serde_json::from_str(&content)?
    } else {
        State::default()
    };

    Ok((state, lock))
}

/// Load state without locking (for read-only operations)
pub fn load_state_readonly() -> Result<State> {
    let path = config::state_path()?;

    if path.exists() {
        let content = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(State::default())
    }
}

/// Save state to file (lock must be held)
pub fn save_state(state: &State, _lock: &FileLock) -> Result<()> {
    let path = config::state_path()?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(state)?;
    fs::write(&path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn save_state_to(state: &State, path: &PathBuf, _lock: &FileLock) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(state)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn load_state_from(path: &PathBuf) -> Result<(State, FileLock)> {
        let lock = FileLock::acquire(path, Duration::from_secs(LOCK_TIMEOUT_SECS))?;
        let state = if path.exists() {
            let content = fs::read_to_string(path)?;
            serde_json::from_str(&content)?
        } else {
            State::default()
        };
        Ok((state, lock))
    }

    #[test]
    fn test_state_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");

        let state = State::default();
        let lock = FileLock::acquire(&path, Duration::from_secs(1)).unwrap();
        save_state_to(&state, &path, &lock).unwrap();
        drop(lock);

        let (loaded, _lock) = load_state_from(&path).unwrap();
        assert_eq!(loaded.version, STATE_VERSION);
    }
}
