use std::sync::{Mutex, MutexGuard};

/// Poison-tolerant locking.
///
/// Previously the whole app used `mutex.lock().unwrap()`. If a panic ever
/// occurred while a lock was held (e.g. inside a spawned packet-processing
/// task), the `Mutex` became *poisoned* and every subsequent `lock().unwrap()`
/// — including the 200 ms header poll and the reset commands — panicked too.
/// Combined with the panic hook that stops WinDivert, a single panic killed
/// packet capture and froze the meter ("crashes and can't handle new info").
///
/// `lock_safe()` recovers the guard from a poisoned lock instead of panicking,
/// so a one-off panic can no longer cascade into a dead app.
pub trait MutexExt<T: ?Sized> {
    fn lock_safe(&self) -> MutexGuard<'_, T>;
}

impl<T: ?Sized> MutexExt<T> for Mutex<T> {
    fn lock_safe(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
