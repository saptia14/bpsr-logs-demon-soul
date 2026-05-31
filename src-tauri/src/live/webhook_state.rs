use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub type WebhookEnabledMutex = Arc<AtomicBool>;

pub fn create_webhook_enabled(initial: bool) -> WebhookEnabledMutex {
    Arc::new(AtomicBool::new(initial))
}

pub fn set_webhook_enabled(state: &WebhookEnabledMutex, enabled: bool) {
    state.store(enabled, Ordering::Relaxed);
}

pub fn is_webhook_enabled(state: &WebhookEnabledMutex) -> bool {
    state.load(Ordering::Relaxed)
}
