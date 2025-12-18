use crate::error::{HeadsupError, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::time::Duration;

/// A guard that holds a file lock. The lock is released when dropped.
pub struct FileLock {
    _file: File,
}

impl FileLock {
    /// Acquire an exclusive lock on a file with timeout
    pub fn acquire(path: &Path, timeout: Duration) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create or open the lock file
        let lock_path = path.with_extension("lock");
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&lock_path)?;

        // Try to acquire lock with timeout
        let start = std::time::Instant::now();
        loop {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(FileLock { _file: file }),
                Err(_) if start.elapsed() < timeout => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(_) => return Err(HeadsupError::StateLocked),
            }
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // Lock is automatically released when file is closed
        let _ = self._file.unlock();
    }
}
