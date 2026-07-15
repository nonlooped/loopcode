//! Shared helpers for the integration harness.

use std::sync::{Mutex, MutexGuard};

/// Serializes tests that touch process-global fixture env vars
/// (`LOOPCODE_FETCH_FIXTURE`, `LOOPCODE_MCP_FIXTURE`). All slices share one
/// process in this harness, so unsynchronized `set_var`/`remove_var` across
/// parallel test threads would race.
pub fn env_guard() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
