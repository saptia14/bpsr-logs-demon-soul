use crate::live::opcodes_models::class;
use crate::live::opcodes_models::{CombatStats, Encounter};
use crate::live::player_state::PlayerCacheMutex;
use log::{error, info};
use reqwest::multipart;
use serde::Serialize;
use serde_json::json;

pub type PendingWebhookState = std::sync::Mutex<Option<crate::live::opcodes_models::Encounter>>;

// URL Hardcodeada solicitada por el usuario (DemonSoul Endpoint)
const DEMONSOUL_WEBHOOK: &str = "https://discord.com/api/webhooks/1482216285071872121/IYJmTeJ4Cyl8afzs5v5Euhz1X7WNsh5q0o9gV-kB2RfMrJZQh77txzH_Xc85iFQvqEQQ";

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
    image_data: Option<Vec<u8>>,
) {
    if !crate::live::webhook_state::is_webhook_enabled(webhook_enabled) {
        info!("Skipping webhook report: webhook disabled in settings");
        return;
    }

    if encounter.time_fight_start_ms == 0 || encounter.dmg_stats.value == 0 {
        info!("Skipping webhook report: empty encounter or no damage");
        return;
    }

    let time_elapsed_ms = encounter.time_last_combat_packet_ms.saturating_sub(encounter.time_fight_start_ms);
    let time_elapsed_secs = time_elapsed_ms as f64 / 1000.0;
    if time_elapsed_secs < 2.0 {
        info!("Skipping webhook report: encounter too short ({}s)", time_elapsed_secs);
        return;
    }

    let local_player_uid = encounter
        .local_player
        .as_ref()
        .and_then(|data| data.v_data.as_ref())
        .map(|v| v.char_id)
        .unwrap_or(0);

    let level_map_id = {
        let state = player_state_mutex.lock().unwrap();
        state.get_level_map_id_opt().unwrap_or(0)
    };

    let mut players_data = Vec::new();
    let mut top_player_name = String::from("Unknown");
    let mut top_player_dmg = 0;
    let mut reporter_name = format!("Player {}", local_player_uid);

    {
        // Scope for the mutex lock so it drops before tokio::spawn
        let player_cache = player_cache_mutex.lock().unwrap();

        if let Some(name) = encounter.entity_uid_to_entity.get(&(local_player_uid as i64)).and_then(|e| e.name.clone()).or_else(|| player_cache.get_name(local_player_uid as i64)) {
            reporter_name = name;
        }

        for (&uid, entity) in &encounter.entity_uid_to_entity {
            if entity.entity_type != crate::protocol::pb::EEntityType::EntChar || entity.dmg_stats.value == 0 {
                continue;
            }

            let name = entity.name.clone().or_else(|| player_cache.get_name(uid)).unwrap_or_else(|| format!("Player {uid}"));
            let class = class::get_class_name(entity.class.or_else(|| player_cache.get_class(uid)).unwrap_or(class::Class::Unknown));
            let class_spec = class::get_class_spec(entity.class_spec.or_else(|| player_cache.get_class_spec(uid)).unwrap_or(class::ClassSpec::Unknown));
            let ability_score = entity.ability_score.or_else(|| player_cache.get_ability_score(uid)).unwrap_or(-1);

            if entity.dmg_stats.value > top_player_dmg {
                top_player_dmg = entity.dmg_stats.value;
                top_player_name = name.clone();
            }

            // Combine DPS and Heal skills
            let mut all_skills: std::collections::HashMap<i32, ZdpsSkillStats> = std::collections::HashMap::new();

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
                b.total_damage.partial_cmp(&a.total_damage)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(b.total_healing.partial_cmp(&a.total_healing).unwrap_or(std::cmp::Ordering::Equal))
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
    players_data.sort_by(|a, b| b.total_damage.partial_cmp(&a.total_damage).unwrap_or(std::cmp::Ordering::Equal));

    let encounter_data = ZdpsEncounterData {
        metadata: ZdpsMetadata {
            duration_seconds: time_elapsed_secs as i64,
            total_damage: encounter.dmg_stats.value as f64,
            total_dps: encounter.dmg_stats.value as f64 / time_elapsed_secs,
            total_healing: encounter.heal_stats.value as f64,
            total_hps: encounter.heal_stats.value as f64 / time_elapsed_secs,
            local_player_uid: local_player_uid as f64,
            reporter_name: reporter_name.clone(),
            level_map_id,
            level_map_name: crate::live::opcodes_models::get_scene_name(level_map_id),
        },
        players: players_data,
    };

    let json_bytes = match serde_json::to_vec_pretty(&encounter_data) {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to serialize zdps_data.json: {}", e);
            return;
        }
    };

    let pretty_dps = format!("{:.2}M", (encounter.dmg_stats.value as f64 / time_elapsed_secs) / 1_000_000.0);
    let pretty_total = format!("{:.2}M", encounter.dmg_stats.value as f64 / 1_000_000.0);
    let pretty_hps = format!("{:.2}K", (encounter.heal_stats.value as f64 / time_elapsed_secs) / 1_000.0);
    let pretty_healing = format!("{:.2}M", encounter.heal_stats.value as f64 / 1_000_000.0);

    let embed = json!({
        "embeds": [{
            "title": "DemonSoul - ZDPS Meter Report",
            "color": 0x9000ff,
            "description": "Combat encounter ended. Raw JSON data attached.",
            "fields": [
                 { "name": "Duration", "value": format!("{}s", time_elapsed_secs as i64), "inline": true },
                 { "name": "Total DPS", "value": pretty_dps, "inline": true },
                 { "name": "Total Damage", "value": pretty_total, "inline": true },
                 { "name": "Total HPS", "value": pretty_hps, "inline": true },
                 { "name": "Total Healing", "value": pretty_healing, "inline": true },
                 { "name": "Top Player", "value": top_player_name, "inline": false },
                 { "name": "Reporter", "value": reporter_name, "inline": true },
                 { "name": "Instance Map", "value": crate::live::opcodes_models::get_scene_name(level_map_id), "inline": true }
            ],
            "footer": { "text": "Powered by BPSR Rust Backend" },
            "timestamp": chrono::Utc::now().to_rfc3339()
        }]
    });

    let embed_json = embed.to_string();

    tauri::async_runtime::spawn(async move {
        let part_json = multipart::Part::bytes(json_bytes)
            .file_name("zdps_data.json")
            .mime_str("application/json")
            .unwrap();

        let part_embed = multipart::Part::text(embed_json)
            .mime_str("application/json")
            .unwrap();

        let mut form = multipart::Form::new()
            .part("files[0]", part_json)
            .part("payload_json", part_embed);

        if let Some(img_bytes) = image_data {
            let part_img = multipart::Part::bytes(img_bytes)
                .file_name("screenshot.png")
                .mime_str("image/png")
                .unwrap();
            form = form.part("files[1]", part_img);
        }

        let client = reqwest::Client::new();
        match client.post(DEMONSOUL_WEBHOOK).multipart(form).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Successfully submitted report to DemonSoul webhook");
                } else {
                    error!("Failed to submit webhook report. Status: {}", response.status());
                }
            }
            Err(e) => {
                error!("Error sending webhook POST request: {}", e);
            }
        }
    });
}
