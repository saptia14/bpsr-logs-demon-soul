//! In-memory chat log + Guild (Union) → Discord relay.
//!
//! Chat arrives as a `ChitChatNtf` Notify (see `artifacts/Chat-Integration-Plan.md`),
//! decoded in `live_main`. Messages are kept in a bounded ring buffer for the UI
//! Chat tab. Guild/Union text messages are forwarded to a **server-side dedupe
//! relay** (`artifacts/Guild-Chat-Dedupe-Endpoint-Spec.md`) as `"Name: text"`,
//! keyed by a deterministic idempotency key derived from game-broadcast fields
//! (identical for every receiver of the same message). The relay holds the
//! Discord webhook and posts exactly once across all users; the client only
//! sends `{idempotencyKey, content}` with an embedded API key.

use crate::utils::sync::MutexExt;
use log::error;
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

/// ChitChatChannelType values (see plan §2.3).
pub const CHANNEL_UNION: i32 = 4;

const MAX_CHAT_HISTORY: usize = 500;
const MAX_DEDUP_KEYS: usize = 2000;

/// Dedupe relay endpoint (not secret — has a default; overridable via env).
const DEFAULT_DEDUPE_ENDPOINT: &str = "https://bpsr.otterteamstudio.com/api/guild-chat/dedupe";
const DEFAULT_HEALTH_ENDPOINT: &str = "https://bpsr.otterteamstudio.com/health";
const COMPILE_TIME_DEDUPE_ENDPOINT: Option<&str> = option_env!("BPSR_DEDUPE_ENDPOINT");
// Secret: never hardcoded. Provided via runtime `.env` (dev) or a compile-time
// CI secret (release builds). Absent => relay silently disabled.
const COMPILE_TIME_DEDUPE_API_KEY: Option<&str> = option_env!("BPSR_DEDUPE_API_KEY");

static DEDUPE_ENDPOINT: LazyLock<String> = LazyLock::new(|| {
    std::env::var("BPSR_DEDUPE_ENDPOINT")
        .ok()
        .or_else(|| COMPILE_TIME_DEDUPE_ENDPOINT.map(String::from))
        .unwrap_or_else(|| DEFAULT_DEDUPE_ENDPOINT.to_string())
});

static HEALTH_ENDPOINT: LazyLock<String> = LazyLock::new(|| {
    std::env::var("BPSR_DEDUPE_HEALTH")
        .ok()
        .unwrap_or_else(|| DEFAULT_HEALTH_ENDPOINT.to_string())
});

static DEDUPE_API_KEY: LazyLock<Option<String>> = LazyLock::new(|| {
    std::env::var("BPSR_DEDUPE_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .or_else(|| COMPILE_TIME_DEDUPE_API_KEY.map(String::from))
});

/// Whether this build has the relay API key baked in / available.
pub fn relay_configured() -> bool {
    DEDUPE_API_KEY.is_some()
}

pub fn channel_name(channel: i32) -> &'static str {
    match channel {
        1 => "World",
        2 => "Local",
        3 => "Team",
        4 => "Union",
        5 => "Private",
        6 => "Group",
        7 => "Top Notice",
        8 => "Play",
        9 => "Newbie",
        99 => "System",
        _ => "Unknown",
    }
}

/// One stored chat message.
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub id: u64, // app-assigned, monotonic
    pub channel: i32,
    pub sender_uid: i64,
    pub sender_name: String,
    pub sender_level: i32,
    pub msg_type: i32,
    pub text: String,
    pub timestamp: i64, // unix seconds (from the game)
    pub game_msg_id: i64,
}

/// Row shipped to the UI.
#[derive(serde::Serialize, specta::Type, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChatRow {
    pub id: f64,
    pub channel: i32,
    pub channel_name: String,
    pub sender_uid: f64,
    pub sender_name: String,
    pub sender_level: i32,
    pub msg_type: i32,
    pub text: String,
    pub timestamp: f64,
}

#[derive(Default)]
pub struct ChatStore {
    messages: VecDeque<ChatMessage>,
    next_id: u64,
}

impl ChatStore {
    /// Append a message, assigning it an app-local id and trimming history.
    pub fn push(&mut self, mut msg: ChatMessage) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        msg.id = id;
        self.messages.push_back(msg);
        while self.messages.len() > MAX_CHAT_HISTORY {
            self.messages.pop_front();
        }
        id
    }

    /// Most-recent `limit` messages (chronological), optionally filtered by
    /// channel (empty `channels` = all).
    pub fn rows(&self, channels: &[i32], limit: usize) -> Vec<ChatRow> {
        let mut rows: Vec<ChatRow> = self
            .messages
            .iter()
            .rev()
            .filter(|m| channels.is_empty() || channels.contains(&m.channel))
            .take(limit)
            .map(|m| ChatRow {
                id: m.id as f64,
                channel: m.channel,
                channel_name: channel_name(m.channel).to_string(),
                sender_uid: m.sender_uid as f64,
                sender_name: m.sender_name.clone(),
                sender_level: m.sender_level,
                msg_type: m.msg_type,
                text: m.text.clone(),
                timestamp: m.timestamp as f64,
            })
            .collect();
        rows.reverse(); // back to chronological order
        rows
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

pub type ChatStoreMutex = Mutex<ChatStore>;

/// Bounded set used for client-side relay dedup.
#[derive(Default)]
struct DedupSet {
    set: HashSet<String>,
    order: VecDeque<String>,
}

impl DedupSet {
    /// Insert `key`; returns true if it was new (i.e. should be relayed).
    fn insert_new(&mut self, key: String) -> bool {
        if self.set.contains(&key) {
            return false;
        }
        self.set.insert(key.clone());
        self.order.push_back(key);
        while self.order.len() > MAX_DEDUP_KEYS {
            if let Some(old) = self.order.pop_front() {
                self.set.remove(&old);
            }
        }
        true
    }
}

/// Guild (Union) chat → dedupe-relay configuration + local pre-dedup state.
/// The Discord webhook lives on the server; this only holds the on/off gate and
/// a small local "already sent this key" filter so a re-delivery on one client
/// doesn't hammer the endpoint.
pub struct GuildRelay {
    enabled: AtomicBool,
    sent_keys: Mutex<DedupSet>,
}

pub type GuildRelayState = Arc<GuildRelay>;

impl GuildRelay {
    pub fn new(enabled: bool) -> GuildRelayState {
        Arc::new(Self {
            enabled: AtomicBool::new(enabled),
            sent_keys: Mutex::new(DedupSet::default()),
        })
    }

    pub fn set(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Record an idempotency key; returns true if it was not seen before.
    fn mark_sent(&self, key: &str) -> bool {
        self.sent_keys.lock_safe().insert_new(key.to_string())
    }
}

/// Deterministic idempotency key from game-broadcast fields. Because every
/// receiver of the same guild message sees the same channel / game msg id /
/// sender / timestamp, this key is identical across all users running the app —
/// which is what lets the server post exactly one copy.
pub fn idempotency_key(channel: i32, game_msg_id: i64, sender_uid: i64, timestamp: i64) -> String {
    format!("{channel}:{game_msg_id}:{sender_uid}:{timestamp}")
}

/// Forward a Guild/Union message to the server-side dedupe relay as
/// `"Name: text"`. No-op when disabled, unconfigured (no API key), empty, or
/// already sent from this instance. The server decides whether to post to
/// Discord (cross-user dedup). Fire-and-forget.
pub fn relay_guild_message(relay: &GuildRelayState, sender_name: &str, text: &str, key: String) {
    if !relay.is_enabled() || text.trim().is_empty() {
        return;
    }
    let Some(api_key) = DEDUPE_API_KEY.clone() else {
        return; // relay not configured in this build
    };
    // Local pre-filter: don't re-send the same key from this instance.
    if !relay.mark_sent(&key) {
        return;
    }

    let content = format!("{sender_name}: {text}");
    let endpoint = DEDUPE_ENDPOINT.clone();
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "idempotencyKey": key,
            "content": content,
            "channel": "union",
        });
        match client
            .post(&endpoint)
            .header("X-API-Key", api_key)
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {}
            Ok(resp) => error!(
                "Guild chat relay: dedupe endpoint returned HTTP {}",
                resp.status()
            ),
            Err(e) => error!("Guild chat relay error: {e}"),
        }
    });
}

/// Relay status for the UI: whether the build has the API key and whether the
/// server `/health` endpoint is reachable.
#[derive(serde::Serialize, specta::Type, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GuildRelayStatus {
    pub configured: bool,
    pub reachable: bool,
}

pub async fn relay_status() -> GuildRelayStatus {
    let configured = relay_configured();
    let reachable = match reqwest::Client::new()
        .get(HEALTH_ENDPOINT.as_str())
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    };
    GuildRelayStatus {
        configured,
        reachable,
    }
}
