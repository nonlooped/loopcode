//! Single-instance process lock — exclusive OS file lock held for process lifetime.

use fs4::fs_std::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SingleInstanceDecision {
    /// This process owns the instance lock.
    Primary,
    /// Another instance already holds the lock — focus existing (caller activates window).
    SecondaryAlreadyRunning,
}

/// Holds an **exclusive** OS lock on the lock file for the process lifetime.
/// Dropping the guard unlocks (and best-effort removes) the file.
pub struct SingleInstanceGuard {
    file: File,
    path: PathBuf,
}

impl SingleInstanceGuard {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
        // Best-effort cleanup so next primary can create a fresh file.
        let _ = fs::remove_file(&self.path);
    }
}

/// Try to acquire an exclusive instance lock at `lock_path`.
///
/// Uses `try_lock_exclusive` (LockFileEx on Windows / flock on Unix) so a second
/// acquire while the first guard is alive returns [`SecondaryAlreadyRunning`].
pub fn acquire_single_instance(
    lock_path: &Path,
) -> Result<(SingleInstanceDecision, Option<SingleInstanceGuard>), String> {
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|e| format!("open single-instance lock: {e}"))?;

    match file.try_lock_exclusive() {
        Ok(()) => {
            // We hold the exclusive lock for this process.
            file.set_len(0).map_err(|e| e.to_string())?;
            writeln!(file, "{}", std::process::id()).map_err(|e| e.to_string())?;
            let _ = file.sync_all();
            Ok((
                SingleInstanceDecision::Primary,
                Some(SingleInstanceGuard {
                    file,
                    path: lock_path.to_path_buf(),
                }),
            ))
        }
        Err(_) => {
            // Another process holds an exclusive lock (or lock unavailable).
            Ok((SingleInstanceDecision::SecondaryAlreadyRunning, None))
        }
    }
}

pub fn release_single_instance(guard: SingleInstanceGuard) -> Result<(), String> {
    // Drop unlocks via Drop impl.
    drop(guard);
    Ok(())
}

pub fn default_lock_path(data_dir: &Path) -> PathBuf {
    data_dir.join("loopcode.single-instance.lock")
}

/// Pure decision helper for tests: given "lock held by other" flag.
pub fn decide_instance(lock_held_by_other: bool) -> SingleInstanceDecision {
    if lock_held_by_other {
        SingleInstanceDecision::SecondaryAlreadyRunning
    } else {
        SingleInstanceDecision::Primary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_then_secondary_while_guard_held() {
        let dir = std::env::temp_dir().join(format!(
            "lc-si-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let lock = dir.join("app.lock");

        let (d1, g1) = acquire_single_instance(&lock).unwrap();
        assert_eq!(d1, SingleInstanceDecision::Primary);
        let g1 = g1.expect("primary must return a live exclusive guard");

        // Second acquire while first exclusive lock is held MUST be secondary.
        let (d2, g2) = acquire_single_instance(&lock).unwrap();
        assert_eq!(
            d2,
            SingleInstanceDecision::SecondaryAlreadyRunning,
            "second acquire while primary guard lives must be Secondary"
        );
        assert!(
            g2.is_none(),
            "secondary must not receive a guard"
        );

        // After primary releases, a new primary can acquire.
        drop(g1);
        let (d3, g3) = acquire_single_instance(&lock).unwrap();
        assert_eq!(d3, SingleInstanceDecision::Primary);
        assert!(g3.is_some());
        drop(g3);

        assert_eq!(
            decide_instance(true),
            SingleInstanceDecision::SecondaryAlreadyRunning
        );
        assert_eq!(decide_instance(false), SingleInstanceDecision::Primary);
    }
}
