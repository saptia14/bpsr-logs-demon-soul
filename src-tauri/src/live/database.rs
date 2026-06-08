//! Local encounter-history database (SQLite, bundled — no system dependency).
//!
//! Every finished encounter (manual reset, inactivity auto-reset, server/scene
//! change) is persisted so past fights can be browsed offline and player names
//! recovered later. Mirrors ZDPS's encounter-history feature. A full per-player
//! / per-skill snapshot is stored as JSON in the `data` column; the summary
//! columns are duplicated for fast listing/sorting without parsing every blob.

use crate::live::opcodes_models::class::{self, Class, ClassSpec};
use crate::live::opcodes_models::{CombatStats, Encounter, get_scene_name};
use crate::live::player_state::{PlayerCache, PlayerState};
use crate::protocol::pb::EEntityType;
use crate::utils::sync::MutexExt;
use log::{error, info, warn};
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::Mutex;

/// Managed state. `None` if the database failed to open (the app still runs,
/// just without history persistence).
pub type DatabaseMutex = Mutex<Option<Connection>>;

/// Minimum encounter length / damage to bother persisting (matches the webhook
/// guard so trivial taps don't flood the history list).
const MIN_DURATION_SECS: f64 = 2.0;

#[derive(serde::Serialize, serde::Deserialize, specta::Type, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoredSkill {
    pub uid: f64,
    pub name: String,
    pub total_damage: f64,
    pub total_healing: f64,
    pub hits: f64,
    pub crit_hits: f64,
    pub crit_value: f64,
    pub lucky_hits: f64,
    pub lucky_value: f64,
}

#[derive(serde::Serialize, serde::Deserialize, specta::Type, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoredPlayer {
    pub uid: f64,
    pub name: String,
    pub class_name: String,
    pub class_spec_name: String,
    pub ability_score: f64,
    pub total_damage: f64,
    pub total_healing: f64,
    pub hits: f64,
    pub crit_hits: f64,
    pub crit_value: f64,
    pub lucky_hits: f64,
    pub lucky_value: f64,
    pub skills: Vec<StoredSkill>,
}

/// JSON blob stored in `encounters.data`.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct StoredSnapshot {
    players: Vec<StoredPlayer>,
}

#[derive(serde::Serialize, serde::Deserialize, specta::Type, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EncounterSummary {
    pub id: f64,
    pub created_at: f64,
    pub duration_ms: f64,
    pub total_damage: f64,
    pub total_dps: f64,
    pub total_healing: f64,
    pub total_hps: f64,
    pub map_id: f64,
    pub map_name: String,
    pub reporter_name: String,
    pub top_player_name: String,
    pub player_count: f64,
}

#[derive(serde::Serialize, serde::Deserialize, specta::Type, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EncounterDetail {
    pub summary: EncounterSummary,
    pub players: Vec<StoredPlayer>,
}

/// Open (and migrate) the encounter database at `path`.
pub fn open(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL").ok();
    conn.pragma_update(None, "synchronous", "NORMAL").ok();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS encounters (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at       INTEGER NOT NULL,
            duration_ms      INTEGER NOT NULL,
            total_damage     INTEGER NOT NULL,
            total_dps        REAL    NOT NULL,
            total_healing    INTEGER NOT NULL,
            total_hps        REAL    NOT NULL,
            map_id           INTEGER NOT NULL,
            map_name         TEXT    NOT NULL,
            local_player_uid INTEGER NOT NULL,
            reporter_name    TEXT    NOT NULL,
            top_player_name  TEXT    NOT NULL,
            player_count     INTEGER NOT NULL,
            data             TEXT    NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_encounters_created_at
            ON encounters(created_at DESC);",
    )?;
    Ok(conn)
}

/// Build the per-player / per-skill snapshot from the live encounter, merging
/// damage and healing skills (same shape the webhook reporter uses).
fn build_players(encounter: &Encounter, player_cache: &PlayerCache) -> Vec<StoredPlayer> {
    let mut players = Vec::new();
    for (&uid, entity) in &encounter.entity_uid_to_entity {
        let did_combat = entity.dmg_stats.value > 0 || entity.heal_stats.value > 0;
        if entity.entity_type != EEntityType::EntChar || !did_combat {
            continue;
        }

        let name = entity
            .name
            .clone()
            .or_else(|| player_cache.get_name(uid))
            .unwrap_or_else(|| format!("Player {uid}"));
        let class_name = class::get_class_name(
            entity
                .class
                .or_else(|| player_cache.get_class(uid))
                .unwrap_or(Class::Unknown),
        );
        let class_spec_name = class::get_class_spec(
            entity
                .class_spec
                .or_else(|| player_cache.get_class_spec(uid))
                .unwrap_or(ClassSpec::Unknown),
        );
        let ability_score = entity
            .ability_score
            .or_else(|| player_cache.get_ability_score(uid))
            .unwrap_or(-1);

        // Merge damage + heal skills by skill uid.
        let mut skills: HashMap<i32, StoredSkill> = HashMap::new();
        for (&skill_uid, s) in &entity.skill_uid_to_dps_stats {
            skills.insert(
                skill_uid,
                StoredSkill {
                    uid: skill_uid as f64,
                    name: CombatStats::get_skill_name(skill_uid),
                    total_damage: s.value as f64,
                    total_healing: 0.0,
                    hits: s.hits as f64,
                    crit_hits: s.crit_hits as f64,
                    crit_value: s.crit_value as f64,
                    lucky_hits: s.lucky_hits as f64,
                    lucky_value: s.lucky_value as f64,
                },
            );
        }
        for (&skill_uid, s) in &entity.skill_uid_to_heal_stats {
            let e = skills.entry(skill_uid).or_insert_with(|| StoredSkill {
                uid: skill_uid as f64,
                name: CombatStats::get_skill_name(skill_uid),
                total_damage: 0.0,
                total_healing: 0.0,
                hits: 0.0,
                crit_hits: 0.0,
                crit_value: 0.0,
                lucky_hits: 0.0,
                lucky_value: 0.0,
            });
            e.total_healing += s.value as f64;
        }
        let mut skills: Vec<StoredSkill> = skills.into_values().collect();
        skills.sort_by(|a, b| {
            b.total_damage
                .partial_cmp(&a.total_damage)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    b.total_healing
                        .partial_cmp(&a.total_healing)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });

        players.push(StoredPlayer {
            uid: uid as f64,
            name,
            class_name,
            class_spec_name,
            ability_score: ability_score as f64,
            total_damage: entity.dmg_stats.value as f64,
            total_healing: entity.heal_stats.value as f64,
            hits: entity.dmg_stats.hits as f64,
            crit_hits: entity.dmg_stats.crit_hits as f64,
            crit_value: entity.dmg_stats.crit_value as f64,
            lucky_hits: entity.dmg_stats.lucky_hits as f64,
            lucky_value: entity.dmg_stats.lucky_value as f64,
            skills,
        });
    }
    players.sort_by(|a, b| {
        b.total_damage
            .partial_cmp(&a.total_damage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    players
}

/// Persist a finished encounter. No-op when the encounter is empty/too short or
/// the database is unavailable. Safe to call from any reset path.
pub fn save_encounter(
    db: &DatabaseMutex,
    encounter: &Encounter,
    player_cache: &PlayerCache,
    player_state: &PlayerState,
) {
    if encounter.time_fight_start_ms == 0 || encounter.dmg_stats.value == 0 {
        return;
    }
    let duration_ms = encounter
        .time_last_combat_packet_ms
        .saturating_sub(encounter.time_fight_start_ms);
    let duration_secs = duration_ms as f64 / 1000.0;
    if duration_secs < MIN_DURATION_SECS {
        return;
    }

    let guard = db.lock_safe();
    let Some(conn) = guard.as_ref() else {
        return; // database disabled
    };

    let players = build_players(encounter, player_cache);
    if players.is_empty() {
        return;
    }

    let map_id = player_state.get_level_map_id_opt().unwrap_or(0);
    let local_uid = player_state.get_uid();
    let reporter_name = players
        .iter()
        .find(|p| p.uid as i64 == local_uid)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| format!("Player {local_uid}"));
    let top_player_name = players
        .first()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let total_damage = encounter.dmg_stats.value;
    let total_healing = encounter.heal_stats.value;
    let total_dps = total_damage as f64 / duration_secs;
    let total_hps = total_healing as f64 / duration_secs;
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let snapshot = StoredSnapshot {
        players: players.clone(),
    };
    let data = match serde_json::to_string(&snapshot) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to serialize encounter snapshot: {e}");
            return;
        }
    };

    let result = conn.execute(
        "INSERT INTO encounters (created_at, duration_ms, total_damage, total_dps,
            total_healing, total_hps, map_id, map_name, local_player_uid,
            reporter_name, top_player_name, player_count, data)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            created_at,
            duration_ms as i64,
            total_damage,
            total_dps,
            total_healing,
            total_hps,
            map_id as i64,
            get_scene_name(map_id),
            local_uid,
            reporter_name,
            top_player_name,
            players.len() as i64,
            data,
        ],
    );
    match result {
        Ok(_) => info!(
            "Saved encounter to history ({} players, {:.0}s)",
            players.len(),
            duration_secs
        ),
        Err(e) => error!("Failed to save encounter to history: {e}"),
    }
}

fn row_to_summary(row: &rusqlite::Row) -> rusqlite::Result<EncounterSummary> {
    Ok(EncounterSummary {
        id: row.get::<_, i64>("id")? as f64,
        created_at: row.get::<_, i64>("created_at")? as f64,
        duration_ms: row.get::<_, i64>("duration_ms")? as f64,
        total_damage: row.get::<_, i64>("total_damage")? as f64,
        total_dps: row.get::<_, f64>("total_dps")?,
        total_healing: row.get::<_, i64>("total_healing")? as f64,
        total_hps: row.get::<_, f64>("total_hps")?,
        map_id: row.get::<_, i64>("map_id")? as f64,
        map_name: row.get("map_name")?,
        reporter_name: row.get("reporter_name")?,
        top_player_name: row.get("top_player_name")?,
        player_count: row.get::<_, i64>("player_count")? as f64,
    })
}

pub fn list_encounters(db: &DatabaseMutex, limit: i64) -> Result<Vec<EncounterSummary>, String> {
    let guard = db.lock_safe();
    let Some(conn) = guard.as_ref() else {
        return Ok(Vec::new());
    };
    let mut stmt = conn
        .prepare(
            "SELECT id, created_at, duration_ms, total_damage, total_dps, total_healing,
                total_hps, map_id, map_name, reporter_name, top_player_name, player_count
             FROM encounters ORDER BY created_at DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([limit], row_to_summary)
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

pub fn get_detail(db: &DatabaseMutex, id: i64) -> Result<EncounterDetail, String> {
    let guard = db.lock_safe();
    let Some(conn) = guard.as_ref() else {
        return Err("Encounter history database is unavailable.".to_string());
    };
    let (summary, data): (EncounterSummary, String) = conn
        .query_row(
            "SELECT id, created_at, duration_ms, total_damage, total_dps, total_healing,
                total_hps, map_id, map_name, reporter_name, top_player_name, player_count, data
             FROM encounters WHERE id = ?1",
            [id],
            |row| Ok((row_to_summary(row)?, row.get::<_, String>("data")?)),
        )
        .map_err(|e| e.to_string())?;
    let snapshot: StoredSnapshot = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(EncounterDetail {
        summary,
        players: snapshot.players,
    })
}

pub fn delete_encounter(db: &DatabaseMutex, id: i64) -> Result<(), String> {
    let guard = db.lock_safe();
    let Some(conn) = guard.as_ref() else {
        return Ok(());
    };
    conn.execute("DELETE FROM encounters WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn clear_all(db: &DatabaseMutex) -> Result<(), String> {
    let guard = db.lock_safe();
    let Some(conn) = guard.as_ref() else {
        return Ok(());
    };
    conn.execute("DELETE FROM encounters", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}
