use crate::live::opcodes_models::class;
use crate::live::opcodes_models::{CombatStats, Encounter};
use crate::live::player_state::PlayerCacheMutex;
use crate::utils::sync::MutexExt;
use log::{error, info};
use serde::Serialize;
use serde_json::json;
use std::sync::LazyLock;

// DPS encounter reports are routed through the server-side dedupe relay (same
// host as the guild-chat relay) instead of posting straight to Discord. The
// relay holds the Discord webhook, renders the graph image from this JSON, and
// posts exactly once across all party members (see
// artifacts/DPS-Report-Dedupe-And-Image-Plan.md). The endpoint has a default;
// the API key is baked in at build time (CI) or supplied via runtime env —
// absent => the relay is silently disabled. The key is shared with the
// guild-chat relay (`BPSR_DEDUPE_API_KEY`).
const DEFAULT_DPS_REPORT_ENDPOINT: &str = "https://bpsr.otterteamstudio.com/api/dps-report/dedupe";
const COMPILE_TIME_DPS_ENDPOINT: Option<&str> = option_env!("BPSR_DPS_REPORT_ENDPOINT");
const COMPILE_TIME_DPS_API_KEY: Option<&str> = option_env!("BPSR_DEDUPE_API_KEY");

static DPS_REPORT_ENDPOINT: LazyLock<String> = LazyLock::new(|| {
    std::env::var("BPSR_DPS_REPORT_ENDPOINT")
        .ok()
        .or_else(|| COMPILE_TIME_DPS_ENDPOINT.map(String::from))
        .unwrap_or_else(|| DEFAULT_DPS_REPORT_ENDPOINT.to_string())
});

static DPS_API_KEY: LazyLock<Option<String>> = LazyLock::new(|| {
    std::env::var("BPSR_DEDUPE_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .or_else(|| COMPILE_TIME_DPS_API_KEY.map(String::from))
});

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ZdpsMetadata {
    duration_seconds: i64,
    total_damage: f64,
    total_dps: f64,
    total_healing: f64,
    total_hps: f64,
    local_player_uid: f64,
    reporter_name: String,
    level_map_id: u32,
    level_map_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ZdpsSkillStats {
    uid: f64,
    name: String,
    total_damage: f64,
    total_healing: f64,
    hits: f64,
    crit_hits: f64,
    crit_value: f64,
    lucky_hits: f64,
    lucky_value: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ZdpsPlayerStats {
    uid: f64,
    name: String,
    class: String,
    class_spec: String,
    ability_score: f64,
    total_damage: f64,
    total_healing: f64,
    total_hps: f64,
    hits: f64,
    crit_hits: f64,
    crit_value: f64,
    lucky_hits: f64,
    lucky_value: f64,
    skills: Vec<ZdpsSkillStats>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ZdpsEncounterData {
    metadata: ZdpsMetadata,
    players: Vec<ZdpsPlayerStats>,
}

pub fn submit_report(
    encounter: &Encounter,
    player_cache_mutex: &PlayerCacheMutex,
    player_state_mutex: &crate::live::player_state::PlayerStateMutex,
    webhook_enabled: &crate::live::webhook_state::WebhookEnabledMutex,
) {
    if !crate::live::webhook_state::is_webhook_enabled(webhook_enabled) {
        info!("Skipping webhook report: webhook disabled in settings");
        return;
    }

    if encounter.time_fight_start_ms == 0 || encounter.dmg_stats.value == 0 {
        info!("Skipping webhook report: empty encounter or no damage");
        return;
    }

    let time_elapsed_ms = encounter
        .time_last_combat_packet_ms
        .saturating_sub(encounter.time_fight_start_ms);
    let time_elapsed_secs = time_elapsed_ms as f64 / 1000.0;
    if time_elapsed_secs < 2.0 {
        info!(
            "Skipping webhook report: encounter too short ({}s)",
            time_elapsed_secs
        );
        return;
    }

    let local_player_uid = encounter
        .local_player
        .as_ref()
        .and_then(|data| data.v_data.as_ref())
        .map(|v| v.char_id)
        .unwrap_or(0);

    let level_map_id = {
        let state = player_state_mutex.lock_safe();
        state.get_level_map_id_opt().unwrap_or(0)
    };

    let mut players_data = Vec::new();
    let mut reporter_name = format!("Player {}", local_player_uid);

    {
        // Scope for the mutex lock so it drops before tokio::spawn
        let player_cache = player_cache_mutex.lock_safe();

        if let Some(name) = encounter
            .entity_uid_to_entity
            .get(&(local_player_uid as i64))
            .and_then(|e| e.name.clone())
            .or_else(|| player_cache.get_name(local_player_uid as i64))
        {
            reporter_name = name;
        }

        for (&uid, entity) in &encounter.entity_uid_to_entity {
            if entity.entity_type != crate::protocol::pb::EEntityType::EntChar
                || entity.dmg_stats.value == 0
            {
                continue;
            }

            let name = entity
                .name
                .clone()
                .or_else(|| player_cache.get_name(uid))
                .unwrap_or_else(|| format!("Player {uid}"));
            let class = class::get_class_name(
                entity
                    .class
                    .or_else(|| player_cache.get_class(uid))
                    .unwrap_or(class::Class::Unknown),
            );
            let class_spec = class::get_class_spec(
                entity
                    .class_spec
                    .or_else(|| player_cache.get_class_spec(uid))
                    .unwrap_or(class::ClassSpec::Unknown),
            );
            let ability_score = entity
                .ability_score
                .or_else(|| player_cache.get_ability_score(uid))
                .unwrap_or(-1);

            // Combine DPS and Heal skills
            let mut all_skills: std::collections::HashMap<i32, ZdpsSkillStats> =
                std::collections::HashMap::new();

            // Insert Damage skills
            for (&skill_uid, skill_stat) in &entity.skill_uid_to_dps_stats {
                all_skills.insert(
                    skill_uid,
                    ZdpsSkillStats {
                        uid: skill_uid as f64,
                        name: CombatStats::get_skill_name(skill_uid),
                        total_damage: skill_stat.value as f64,
                        total_healing: 0.0,
                        hits: skill_stat.hits as f64,
                        crit_hits: skill_stat.crit_hits as f64,
                        crit_value: skill_stat.crit_value as f64,
                        lucky_hits: skill_stat.lucky_hits as f64,
                        lucky_value: skill_stat.lucky_value as f64,
                    },
                );
            }

            // Insert or Update Heal skills
            for (&skill_uid, skill_stat) in &entity.skill_uid_to_heal_stats {
                if let Some(existing) = all_skills.get_mut(&skill_uid) {
                    existing.total_healing += skill_stat.value as f64;
                } else {
                    all_skills.insert(
                        skill_uid,
                        ZdpsSkillStats {
                            uid: skill_uid as f64,
                            name: CombatStats::get_skill_name(skill_uid),
                            total_damage: 0.0,
                            total_healing: skill_stat.value as f64,
                            hits: skill_stat.hits as f64,
                            crit_hits: skill_stat.crit_hits as f64,
                            crit_value: skill_stat.crit_value as f64,
                            lucky_hits: skill_stat.lucky_hits as f64,
                            lucky_value: skill_stat.lucky_value as f64,
                        },
                    );
                }
            }

            let mut skills: Vec<ZdpsSkillStats> = all_skills.into_values().collect();
            // Sort skills mostly by damage, then healing
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

            players_data.push(ZdpsPlayerStats {
                uid: uid as f64,
                name,
                class,
                class_spec,
                ability_score: ability_score as f64,
                total_damage: entity.dmg_stats.value as f64,
                total_healing: entity.heal_stats.value as f64,
                total_hps: entity.heal_stats.value as f64 / time_elapsed_secs,
                hits: entity.dmg_stats.hits as f64,
                crit_hits: entity.dmg_stats.crit_hits as f64,
                crit_value: entity.dmg_stats.crit_value as f64,
                lucky_hits: entity.dmg_stats.lucky_hits as f64,
                lucky_value: entity.dmg_stats.lucky_value as f64,
                skills,
            });
        }
    } // MutexGuard is dropped here

    // Ordenar jugadores por daño descendente
    players_data.sort_by(|a, b| {
        b.total_damage
            .partial_cmp(&a.total_damage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let encounter_data = ZdpsEncounterData {
        metadata: ZdpsMetadata {
            duration_seconds: time_elapsed_secs as i64,
            total_damage: encounter.dmg_stats.value as f64,
            total_dps: encounter.dmg_stats.value as f64 / time_elapsed_secs,
            total_healing: encounter.heal_stats.value as f64,
            total_hps: encounter.heal_stats.value as f64 / time_elapsed_secs,
            local_player_uid: local_player_uid as f64,
            reporter_name,
            level_map_id,
            level_map_name: crate::live::opcodes_models::get_scene_name(level_map_id),
        },
        players: players_data,
    };

    // Idempotency key: deterministic across every party member reporting the
    // same fight. The UID sum is commutative (order-independent), so each
    // client computes the same key regardless of iteration order, and the
    // server posts exactly one copy. `mapId` keeps different zones distinct.
    // See artifacts/DPS-Report-Dedupe-And-Image-Plan.md §4.3.
    let uid_sum: i64 = encounter_data.players.iter().map(|p| p.uid as i64).sum();
    let idempotency_key = format!("dps:{level_map_id}:{uid_sum}");

    let Some(api_key) = DPS_API_KEY.clone() else {
        info!("Skipping DPS report: relay API key not configured in this build");
        return;
    };
    let endpoint = DPS_REPORT_ENDPOINT.clone();

    let encounter_value = match serde_json::to_value(&encounter_data) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to serialize encounter for DPS report: {}", e);
            return;
        }
    };
    let body = json!({
        "idempotencyKey": idempotency_key,
        "encounter": encounter_value,
    });

    // Fire-and-forget: the relay dedupes across users, renders the graph image,
    // and posts to Discord once. Status is only logged locally.
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::new();
        match client
            .post(&endpoint)
            .header("X-API-Key", api_key)
            .json(&body)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                info!("Submitted DPS report to dedupe relay");
            }
            Ok(response) => {
                error!("DPS report relay returned HTTP {}", response.status());
            }
            Err(e) => {
                error!("DPS report relay request error: {}", e);
            }
        }
    });
}
