use crate::live::bptimer_state::{
    BPTimerEnabledMutex, set_bptimer_enabled as update_bptimer_state,
};
use crate::live::commands_models::{HeaderInfo, PlayerRow, PlayersWindow, SkillRow, SkillsWindow};
use crate::live::database::{self, DatabaseMutex, EncounterDetail, EncounterSummary};
use crate::live::module_optimizer::{ModuleSolution, ModulesMutex, OptimizeRequest, optimize};
use crate::live::opcodes_models::class::{Class, ClassSpec};
use crate::live::opcodes_models::{CombatStats, Encounter, EncounterMutex, class};
use crate::live::player_state::{PlayerCacheMutex, PlayerStateMutex};
use crate::packets::packet_capture::request_restart;
use crate::protocol::pb::EEntityType;
use crate::utils::modules::{AttrMeta, ModuleInfo, attribute_list};
use crate::utils::sync::MutexExt;
use log::info;
use std::sync::MutexGuard;

fn nan_is_zero(value: f64) -> f64 {
    if value.is_nan() || value.is_infinite() {
        0.0
    } else {
        value
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_header_info(state: tauri::State<'_, EncounterMutex>) -> HeaderInfo {
    let encounter = state.lock_safe();
    if encounter.dmg_stats.value == 0 {
        return HeaderInfo {
            total_dps: 0.0,
            total_dmg: 0.0,
            elapsed_ms: 0.0,
            time_last_combat_packet_ms: 0.0,
        };
    }

    let time_elapsed_ms = encounter.time_last_combat_packet_ms - encounter.time_fight_start_ms;
    let time_elapsed_secs = time_elapsed_ms as f64 / 1000.0;

    let encounter_stats = &encounter.dmg_stats;

    HeaderInfo {
        total_dps: nan_is_zero(encounter_stats.value as f64 / time_elapsed_secs),
        total_dmg: encounter_stats.value as f64,
        elapsed_ms: time_elapsed_ms as f64,
        time_last_combat_packet_ms: encounter.time_last_combat_packet_ms as f64,
    }
}

#[tauri::command]
#[specta::specta]
pub fn hard_reset(
    state: tauri::State<'_, EncounterMutex>,
    player_cache_state: tauri::State<'_, PlayerCacheMutex>,
    player_state: tauri::State<'_, crate::live::player_state::PlayerStateMutex>,
    webhook_state: tauri::State<'_, crate::live::webhook_state::WebhookEnabledMutex>,
    db: tauri::State<'_, DatabaseMutex>,
) {
    let mut encounter = state.lock_safe();

    database::save_encounter(
        &db,
        &encounter,
        &player_cache_state.lock_safe(),
        &player_state.lock_safe(),
    );
    crate::live::webhook::submit_report(
        &encounter,
        &player_cache_state,
        &player_state,
        &webhook_state,
        None,
    );

    encounter.clone_from(&Encounter::default());
    request_restart();
    info!("Hard Reset");
}

#[tauri::command]
#[specta::specta]
pub fn submit_pending_webhook(
    pending_state: tauri::State<'_, crate::live::webhook::PendingWebhookState>,
    player_cache_state: tauri::State<'_, PlayerCacheMutex>,
    player_state: tauri::State<'_, crate::live::player_state::PlayerStateMutex>,
    webhook_state: tauri::State<'_, crate::live::webhook_state::WebhookEnabledMutex>,
    image: Vec<u8>,
) {
    let mut pending = pending_state.lock_safe();
    if let Some(encounter) = pending.take() {
        crate::live::webhook::submit_report(
            &encounter,
            &player_cache_state,
            &player_state,
            &webhook_state,
            Some(image),
        );
    }
}

#[tauri::command]
#[specta::specta]
pub fn reset_encounter_with_image(
    state: tauri::State<'_, EncounterMutex>,
    player_cache_state: tauri::State<'_, PlayerCacheMutex>,
    player_state: tauri::State<'_, crate::live::player_state::PlayerStateMutex>,
    webhook_state: tauri::State<'_, crate::live::webhook_state::WebhookEnabledMutex>,
    db: tauri::State<'_, DatabaseMutex>,
    image: Vec<u8>,
) {
    let mut encounter = state.lock_safe();

    database::save_encounter(
        &db,
        &encounter,
        &player_cache_state.lock_safe(),
        &player_state.lock_safe(),
    );
    crate::live::webhook::submit_report(
        &encounter,
        &player_cache_state,
        &player_state,
        &webhook_state,
        Some(image),
    );

    encounter.clone_from(&Encounter::default());
    info!("encounter reset with image");
}

#[tauri::command]
#[specta::specta]
pub fn reset_encounter(
    state: tauri::State<'_, EncounterMutex>,
    player_cache_state: tauri::State<'_, PlayerCacheMutex>,
    player_state: tauri::State<'_, crate::live::player_state::PlayerStateMutex>,
    webhook_state: tauri::State<'_, crate::live::webhook_state::WebhookEnabledMutex>,
    db: tauri::State<'_, DatabaseMutex>,
) {
    let mut encounter = state.lock_safe();

    database::save_encounter(
        &db,
        &encounter,
        &player_cache_state.lock_safe(),
        &player_state.lock_safe(),
    );
    crate::live::webhook::submit_report(
        &encounter,
        &player_cache_state,
        &player_state,
        &webhook_state,
        None,
    );

    encounter.clone_from(&Encounter::default());
    info!("encounter reset");
}

#[tauri::command]
#[specta::specta]
pub fn toggle_pause_encounter(state: tauri::State<'_, EncounterMutex>) {
    let mut encounter = state.lock_safe();
    encounter.is_encounter_paused = !encounter.is_encounter_paused;
}

#[tauri::command]
#[specta::specta]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[derive(Debug, Clone, Copy)]
pub enum StatType {
    Dmg,
    DmgBossOnly,
    Heal,
}

#[tauri::command]
#[specta::specta]
pub fn get_dps_player_window(
    state: tauri::State<'_, EncounterMutex>,
    player_cache_state: tauri::State<'_, PlayerCacheMutex>,
    player_state: tauri::State<'_, PlayerStateMutex>,
) -> PlayersWindow {
    let player_state = player_state.lock_safe();
    let encounter = state.lock_safe();
    let player_cache = player_cache_state.lock_safe();
    get_player_window(encounter, StatType::Dmg, &player_cache, &player_state)
}

#[tauri::command]
#[specta::specta]
pub fn get_heal_player_window(
    state: tauri::State<'_, EncounterMutex>,
    player_cache_state: tauri::State<'_, PlayerCacheMutex>,
    player_state: tauri::State<'_, PlayerStateMutex>,
) -> PlayersWindow {
    let player_state = player_state.lock_safe();
    let encounter = state.lock_safe();
    let player_cache = player_cache_state.lock_safe();
    get_player_window(encounter, StatType::Heal, &player_cache, &player_state)
}

#[tauri::command]
#[specta::specta]
pub fn get_dps_boss_only_player_window(
    state: tauri::State<'_, EncounterMutex>,
    player_cache_state: tauri::State<'_, PlayerCacheMutex>,
    player_state: tauri::State<'_, PlayerStateMutex>,
) -> PlayersWindow {
    let player_state = player_state.lock_safe();
    let encounter = state.lock_safe();
    let player_cache = player_cache_state.lock_safe();
    get_player_window(
        encounter,
        StatType::DmgBossOnly,
        &player_cache,
        &player_state,
    )
}

pub fn get_player_window(
    encounter: MutexGuard<Encounter>,
    stat_type: StatType,
    player_cache: &std::sync::MutexGuard<crate::live::player_state::PlayerCache>,
    player_state: &std::sync::MutexGuard<crate::live::player_state::PlayerState>,
) -> PlayersWindow {
    let time_elapsed_ms = encounter.time_last_combat_packet_ms - encounter.time_fight_start_ms;
    let time_elapsed_secs = time_elapsed_ms as f64 / 1000.0;

    let mut player_window = PlayersWindow {
        player_rows: Vec::new(),
        local_player_uid: player_state.get_uid() as f64,
        top_value: 0.0,
    };
    for (&entity_uid, entity) in &encounter.entity_uid_to_entity {
        // Select stats per player and encounter
        let (entity_stats, encounter_stats) = match stat_type {
            StatType::Dmg => (&entity.dmg_stats, &encounter.dmg_stats),
            StatType::DmgBossOnly => (&entity.dmg_stats_boss_only, &encounter.dmg_stats_boss_only),
            StatType::Heal => (&entity.heal_stats, &encounter.heal_stats),
        };
        let is_player = entity.entity_type == EEntityType::EntChar;
        let did_damage = entity_stats.value > 0;
        if !is_player || !did_damage {
            continue;
        }
        player_window.top_value = player_window.top_value.max(entity_stats.value as f64);
        let damage_row = PlayerRow {
            uid: entity_uid as f64,
            name: entity
                .name
                .clone()
                .or_else(|| player_cache.get_name(entity_uid))
                .unwrap_or_else(|| format!("Player {entity_uid}")),
            class_name: class::get_class_name(
                entity
                    .class
                    .or_else(|| player_cache.get_class(entity_uid))
                    .unwrap_or(Class::Unknown),
            ),
            class_spec_name: class::get_class_spec(
                entity
                    .class_spec
                    .or_else(|| player_cache.get_class_spec(entity_uid))
                    .unwrap_or(ClassSpec::Unknown),
            ),
            ability_score: f64::from(
                entity
                    .ability_score
                    .or_else(|| player_cache.get_ability_score(entity_uid))
                    .unwrap_or(-1),
            ),
            total_value: entity_stats.value as f64,
            value_per_sec: nan_is_zero(entity_stats.value as f64 / time_elapsed_secs),
            value_pct: nan_is_zero(
                entity_stats.value as f64 / encounter_stats.value as f64 * 100.0,
            ),
            crit_rate: nan_is_zero(
                entity_stats.crit_hits as f64 / entity_stats.hits as f64 * 100.0,
            ),
            crit_value_rate: nan_is_zero(
                entity_stats.crit_value as f64 / entity_stats.value as f64 * 100.0,
            ),
            lucky_rate: nan_is_zero(
                entity_stats.lucky_hits as f64 / entity_stats.hits as f64 * 100.0,
            ),
            lucky_value_rate: nan_is_zero(
                entity_stats.lucky_value as f64 / entity_stats.value as f64 * 100.0,
            ),
            hits: entity_stats.hits as f64,
            hits_per_minute: nan_is_zero(entity_stats.hits as f64 / time_elapsed_secs * 60.0),
        };
        player_window.player_rows.push(damage_row);
    }
    drop(encounter); // drop lock before expensive sort

    // Sort skills descending by damage dealt
    player_window.player_rows.sort_by(|this_row, other_row| {
        other_row
            .total_value
            .partial_cmp(&this_row.total_value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    player_window
}

#[tauri::command]
#[specta::specta]
pub fn get_dps_skill_window(
    state: tauri::State<'_, EncounterMutex>,
    player_cache_state: tauri::State<'_, PlayerCacheMutex>,
    player_state: tauri::State<'_, PlayerStateMutex>,
    player_uid_str: &str,
) -> Result<SkillsWindow, String> {
    let player_uid = player_uid_str.parse().unwrap();
    let player_state = player_state.lock_safe();
    let encounter = state.lock_safe();
    let player_cache = player_cache_state.lock_safe();
    get_skill_window(
        encounter,
        player_uid,
        StatType::Dmg,
        &player_cache,
        &player_state,
    )
}

#[tauri::command]
#[specta::specta]
pub fn get_dps_boss_only_skill_window(
    state: tauri::State<'_, EncounterMutex>,
    player_cache_state: tauri::State<'_, PlayerCacheMutex>,
    player_state: tauri::State<'_, PlayerStateMutex>,
    player_uid_str: &str,
) -> Result<SkillsWindow, String> {
    let player_uid = player_uid_str.parse().unwrap();
    let player_state = player_state.lock_safe();
    let encounter = state.lock_safe();
    let player_cache = player_cache_state.lock_safe();
    get_skill_window(
        encounter,
        player_uid,
        StatType::DmgBossOnly,
        &player_cache,
        &player_state,
    )
}

#[tauri::command]
#[specta::specta]
pub fn get_heal_skill_window(
    state: tauri::State<'_, EncounterMutex>,
    player_cache_state: tauri::State<'_, PlayerCacheMutex>,
    player_state: tauri::State<'_, PlayerStateMutex>,
    player_uid_str: &str,
) -> Result<SkillsWindow, String> {
    let player_uid = player_uid_str.parse().unwrap();
    let player_state = player_state.lock_safe();
    let encounter = state.lock_safe();
    let player_cache = player_cache_state.lock_safe();
    get_skill_window(
        encounter,
        player_uid,
        StatType::Heal,
        &player_cache,
        &player_state,
    )
}

pub fn get_skill_window(
    encounter: MutexGuard<Encounter>,
    player_uid: i64,
    stat_type: StatType,
    player_cache: &std::sync::MutexGuard<crate::live::player_state::PlayerCache>,
    player_state: &std::sync::MutexGuard<crate::live::player_state::PlayerState>,
) -> Result<SkillsWindow, String> {
    let Some(player) = encounter.entity_uid_to_entity.get(&player_uid) else {
        return Err(format!("Could not find player with uid {player_uid}"));
    };

    let time_elapsed_ms = encounter.time_last_combat_packet_ms - encounter.time_fight_start_ms;
    let time_elapsed_secs = time_elapsed_ms as f64 / 1000.0;

    let (player_stats, encounter_stats, skill_uid_to_stats) = match stat_type {
        StatType::Dmg => (
            &player.dmg_stats,
            &encounter.dmg_stats,
            &player.skill_uid_to_dps_stats,
        ),
        StatType::DmgBossOnly => (
            &player.dmg_stats_boss_only,
            &encounter.dmg_stats_boss_only,
            &player.skill_uid_to_dps_stats_boss_only,
        ),
        StatType::Heal => (
            &player.heal_stats,
            &encounter.heal_stats,
            &player.skill_uid_to_heal_stats,
        ),
    };

    // Player DPS Stats
    let mut skill_window = SkillsWindow {
        inspected_player: PlayerRow {
            uid: player_uid as f64,
            name: player
                .name
                .clone()
                .or_else(|| player_cache.get_name(player_uid))
                .unwrap_or_else(|| format!("Player {player_uid}")),
            class_name: class::get_class_name(
                player
                    .class
                    .or_else(|| player_cache.get_class(player_uid))
                    .unwrap_or(Class::Unknown),
            ),
            class_spec_name: class::get_class_spec(
                player
                    .class_spec
                    .or_else(|| player_cache.get_class_spec(player_uid))
                    .unwrap_or(ClassSpec::Unknown),
            ),
            ability_score: f64::from(
                player
                    .ability_score
                    .or_else(|| player_cache.get_ability_score(player_uid))
                    .unwrap_or(-1),
            ),
            total_value: player_stats.value as f64,
            value_per_sec: nan_is_zero(player_stats.value as f64 / time_elapsed_secs),
            value_pct: nan_is_zero(
                player_stats.value as f64 / encounter_stats.value as f64 * 100.0,
            ),
            crit_rate: nan_is_zero(
                player_stats.crit_hits as f64 / player_stats.hits as f64 * 100.0,
            ),
            crit_value_rate: nan_is_zero(
                player_stats.crit_value as f64 / player_stats.value as f64 * 100.0,
            ),
            lucky_rate: nan_is_zero(
                player_stats.lucky_hits as f64 / player_stats.hits as f64 * 100.0,
            ),
            lucky_value_rate: nan_is_zero(
                player_stats.lucky_value as f64 / player_stats.value as f64 * 100.0,
            ),
            hits: player_stats.hits as f64,
            hits_per_minute: nan_is_zero(player_stats.hits as f64 / time_elapsed_secs * 60.0),
        },
        local_player_uid: player_state.get_uid() as f64,
        skill_rows: Vec::new(),
        top_value: 0.0,
    };

    // Skills for this player
    for (&skill_uid, skill_stat) in skill_uid_to_stats {
        skill_window.top_value = skill_window.top_value.max(skill_stat.value as f64);
        let skill_row = SkillRow {
            uid: f64::from(skill_uid),
            name: CombatStats::get_skill_name(skill_uid),
            total_value: skill_stat.value as f64,
            value_per_sec: nan_is_zero(skill_stat.value as f64 / time_elapsed_secs),
            value_pct: nan_is_zero(skill_stat.value as f64 / player_stats.value as f64 * 100.0),
            crit_rate: nan_is_zero(skill_stat.crit_hits as f64 / skill_stat.hits as f64 * 100.0),
            crit_value_rate: nan_is_zero(
                skill_stat.crit_value as f64 / skill_stat.value as f64 * 100.0,
            ),
            lucky_rate: nan_is_zero(skill_stat.lucky_hits as f64 / skill_stat.hits as f64 * 100.0),
            lucky_value_rate: nan_is_zero(
                skill_stat.lucky_value as f64 / skill_stat.value as f64 * 100.0,
            ),
            hits: skill_stat.hits as f64,
            hits_per_minute: nan_is_zero(skill_stat.hits as f64 / time_elapsed_secs * 60.0),
        };
        skill_window.skill_rows.push(skill_row);
    }
    drop(encounter); // drop before expensive sort

    // Sort skills descending by damage dealt
    skill_window.skill_rows.sort_by(|this_row, other_row| {
        other_row
            .total_value
            .partial_cmp(&this_row.total_value) // descending
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(skill_window)
}

#[tauri::command]
#[specta::specta]
pub fn get_test_player_window() -> PlayersWindow {
    PlayersWindow {
        player_rows: vec![
            PlayerRow {
                uid: 10000001.0,
                name: "Name Stormblade (You)".to_string(),
                class_name: "Stormblade".to_string(),
                class_spec_name: String::new(),
                ability_score: 1500.0,
                total_value: 100000.0,
                value_per_sec: 10000.6,
                value_pct: 100.0,
                crit_rate: 0.25,
                crit_value_rate: 2.0,
                lucky_rate: 0.10,
                lucky_value_rate: 1.5,
                hits: 200.0,
                hits_per_minute: 3.3,
            },
            PlayerRow {
                uid: 10000002.0,
                name: "Name Frost Mage".to_string(),
                class_name: "Frost Mage".to_string(),
                class_spec_name: String::new(),
                ability_score: 1500.0,
                total_value: 90000.0,
                value_per_sec: 6000.6,
                value_pct: 90.0,
                crit_rate: 0.25,
                crit_value_rate: 2.0,
                lucky_rate: 0.10,
                lucky_value_rate: 1.5,
                hits: 200.0,
                hits_per_minute: 3.3,
            },
            PlayerRow {
                uid: 10000003.0,
                name: "Name Wind Knight".to_string(),
                class_name: "Wind Knight".to_string(),
                class_spec_name: String::new(),
                ability_score: 1500.0,
                total_value: 80000.0,
                value_per_sec: 6000.6,
                value_pct: 80.0,
                crit_rate: 0.25,
                crit_value_rate: 2.0,
                lucky_rate: 0.10,
                lucky_value_rate: 1.5,
                hits: 200.0,
                hits_per_minute: 3.3,
            },
            PlayerRow {
                uid: 10000004.0,
                name: "Name Verdant Oracle".to_string(),
                class_name: "Verdant Oracle".to_string(),
                class_spec_name: String::new(),
                ability_score: 1500.0,
                total_value: 70000.0,
                value_per_sec: 6000.6,
                value_pct: 70.0,
                crit_rate: 0.25,
                crit_value_rate: 2.0,
                lucky_rate: 0.10,
                lucky_value_rate: 1.5,
                hits: 200.0,
                hits_per_minute: 3.3,
            },
            PlayerRow {
                uid: 10000005.0,
                name: "Name Heavy Guardian".to_string(),
                class_name: "Heavy Guardian".to_string(),
                class_spec_name: String::new(),
                ability_score: 1500.0,
                total_value: 60000.0,
                value_per_sec: 6000.6,
                value_pct: 60.0,
                crit_rate: 0.25,
                crit_value_rate: 2.0,
                lucky_rate: 0.10,
                lucky_value_rate: 1.5,
                hits: 200.0,
                hits_per_minute: 3.3,
            },
            PlayerRow {
                uid: 10000006.0,
                name: "Name Marksman".to_string(),
                class_name: "Marksman".to_string(),
                class_spec_name: String::new(),
                ability_score: 1500.0,
                total_value: 60000.0,
                value_per_sec: 6000.6,
                value_pct: 50.0,
                crit_rate: 0.25,
                crit_value_rate: 2.0,
                lucky_rate: 0.10,
                lucky_value_rate: 1.5,
                hits: 200.0,
                hits_per_minute: 3.3,
            },
            PlayerRow {
                uid: 10000007.0,
                name: "Name Shield Knight".to_string(),
                class_name: "Shield Knight".to_string(),
                class_spec_name: String::new(),
                ability_score: 1500.0,
                total_value: 50000.0,
                value_per_sec: 6000.6,
                value_pct: 40.0,
                crit_rate: 0.25,
                crit_value_rate: 2.0,
                lucky_rate: 0.10,
                lucky_value_rate: 1.5,
                hits: 200.0,
                hits_per_minute: 3.3,
            },
            PlayerRow {
                uid: 10000008.0,
                name: "Name Beat Performer".to_string(),
                class_name: "Beat Performer".to_string(),
                class_spec_name: String::new(),
                ability_score: 1500.0,
                total_value: 10000.0,
                value_per_sec: 6000.6,
                value_pct: 30.0,
                crit_rate: 0.25,
                crit_value_rate: 2.0,
                lucky_rate: 0.10,
                lucky_value_rate: 1.5,
                hits: 200.0,
                hits_per_minute: 3.3,
            },
            PlayerRow {
                uid: 10000009.0,
                name: "Blank Class".to_string(),
                class_name: "blank".to_string(),
                class_spec_name: String::new(),
                ability_score: 1500.0,
                total_value: 10000.0,
                value_per_sec: 6000.6,
                value_pct: 20.0,
                crit_rate: 0.25,
                crit_value_rate: 2.0,
                lucky_rate: 0.10,
                lucky_value_rate: 1.5,
                hits: 200.0,
                hits_per_minute: 3.3,
            },
        ],
        local_player_uid: 10000001.0,
        top_value: 100000.0,
    }
}

#[tauri::command]
#[specta::specta]
pub fn set_bptimer_enabled(state: tauri::State<BPTimerEnabledMutex>, enabled: bool) {
    update_bptimer_state(&state, enabled);
    info!(
        "BPTimer integration {} via settings",
        if enabled { "enabled" } else { "disabled" }
    );
}

#[tauri::command]
#[specta::specta]
pub fn set_webhook_enabled(
    state: tauri::State<crate::live::webhook_state::WebhookEnabledMutex>,
    enabled: bool,
) {
    crate::live::webhook_state::set_webhook_enabled(&state, enabled);
    info!(
        "Discord webhook {} via settings",
        if enabled { "enabled" } else { "disabled" }
    );
}

#[tauri::command]
#[specta::specta]
pub fn get_test_skill_window(_player_uid: String) -> Result<SkillsWindow, String> {
    Ok(SkillsWindow {
        inspected_player: PlayerRow {
            uid: 10000001.0,
            name: "Name Stormblade".to_string(),
            class_name: "Stormblade".to_string(),
            class_spec_name: "Iaido".to_string(),
            ability_score: 1500.0,
            total_value: 100000.0,
            value_per_sec: 10000.6,
            value_pct: 90.0,
            crit_rate: 0.25,
            crit_value_rate: 2.0,
            lucky_rate: 0.10,
            lucky_value_rate: 1.5,
            hits: 200.0,
            hits_per_minute: 3.3,
        },
        skill_rows: vec![
            SkillRow {
                uid: 3602.0,
                name: "Skill 1".to_string(),
                total_value: 100000.0,
                value_per_sec: 5000.0,
                value_pct: 80.0,
                crit_rate: 0.30,
                crit_value_rate: 2.1,
                lucky_rate: 0.12,
                lucky_value_rate: 1.4,
                hits: 80.0,
                hits_per_minute: 1.5,
            },
            SkillRow {
                uid: 3602.0,
                name: "Skill 2".to_string(),
                total_value: 50000.0,
                value_per_sec: 7345.6,
                value_pct: 70.0,
                crit_rate: 0.20,
                crit_value_rate: 1.9,
                lucky_rate: 0.08,
                lucky_value_rate: 1.3,
                hits: 120.0,
                hits_per_minute: 1.8,
            },
            SkillRow {
                uid: 3602.0,
                name: "Skill 3".to_string(),
                total_value: 33000.0,
                value_per_sec: 7345.6,
                value_pct: 60.0,
                crit_rate: 0.20,
                crit_value_rate: 1.9,
                lucky_rate: 0.08,
                lucky_value_rate: 1.3,
                hits: 120.0,
                hits_per_minute: 1.8,
            },
            SkillRow {
                uid: 3602.0,
                name: "Skill 4".to_string(),
                total_value: 23000.0,
                value_per_sec: 7345.6,
                value_pct: 50.0,
                crit_rate: 0.20,
                crit_value_rate: 1.9,
                lucky_rate: 0.08,
                lucky_value_rate: 1.3,
                hits: 120.0,
                hits_per_minute: 1.8,
            },
            SkillRow {
                uid: 3602.0,
                name: "Skill 5".to_string(),
                total_value: 11000.0,
                value_per_sec: 7345.6,
                value_pct: 40.0,
                crit_rate: 0.20,
                crit_value_rate: 1.9,
                lucky_rate: 0.08,
                lucky_value_rate: 1.3,
                hits: 120.0,
                hits_per_minute: 1.8,
            },
            SkillRow {
                uid: 3602.0,
                name: "Skill 6".to_string(),
                total_value: 1000.0,
                value_per_sec: 7345.6,
                value_pct: 30.0,
                crit_rate: 0.20,
                crit_value_rate: 1.9,
                lucky_rate: 0.08,
                lucky_value_rate: 1.3,
                hits: 120.0,
                hits_per_minute: 1.8,
            },
            SkillRow {
                uid: 3602.0,
                name: "Skill 7".to_string(),
                total_value: 400.0,
                value_per_sec: 7345.6,
                value_pct: 20.0,
                crit_rate: 0.20,
                crit_value_rate: 1.9,
                lucky_rate: 0.08,
                lucky_value_rate: 1.3,
                hits: 120.0,
                hits_per_minute: 1.8,
            },
        ],
        local_player_uid: 10000001.0,
        top_value: 100000.0,
    })
}

/// List the local player's parsed gear modules. Empty until the game has sent
/// the player's container snapshot (the UI prompts the user to open their
/// character / re-log if so).
#[tauri::command]
#[specta::specta]
pub fn get_modules(modules_state: tauri::State<'_, ModulesMutex>) -> Vec<ModuleInfo> {
    modules_state.lock_safe().clone()
}

/// The list of selectable module attributes (for the optimizer's pickers).
#[tauri::command]
#[specta::specta]
pub fn get_module_attributes() -> Vec<AttrMeta> {
    attribute_list()
}

/// Run the built-in module optimizer over the local player's modules.
#[tauri::command]
#[specta::specta]
pub async fn optimize_modules(
    modules_state: tauri::State<'_, ModulesMutex>,
    request: OptimizeRequest,
) -> Result<Vec<ModuleSolution>, String> {
    // Snapshot the modules under the lock, then run the (potentially heavy)
    // search off the async runtime so the UI stays responsive.
    let modules = modules_state.lock_safe().clone();

    if modules.is_empty() {
        return Err("No module data yet. Open your character / re-log in.".to_string());
    }

    tauri::async_runtime::spawn_blocking(move || optimize(&modules, &request))
        .await
        .map_err(|e| format!("Optimizer task failed: {e}"))
}

/// Browse stored past encounters (most recent first).
#[tauri::command]
#[specta::specta]
pub fn get_encounter_history(
    db: tauri::State<'_, DatabaseMutex>,
    limit: i64,
) -> Result<Vec<EncounterSummary>, String> {
    let limit = if limit <= 0 { 200 } else { limit.min(1000) };
    database::list_encounters(&db, limit)
}

/// Full per-player / per-skill detail for one stored encounter.
#[tauri::command]
#[specta::specta]
pub fn get_encounter_detail(
    db: tauri::State<'_, DatabaseMutex>,
    id: i64,
) -> Result<EncounterDetail, String> {
    database::get_detail(&db, id)
}

/// Delete a single stored encounter.
#[tauri::command]
#[specta::specta]
pub fn delete_encounter(db: tauri::State<'_, DatabaseMutex>, id: i64) -> Result<(), String> {
    database::delete_encounter(&db, id)
}

/// Delete all stored encounters.
#[tauri::command]
#[specta::specta]
pub fn clear_encounter_history(db: tauri::State<'_, DatabaseMutex>) -> Result<(), String> {
    database::clear_all(&db)
}

/// Capture connectivity diagnostics: whether the game process is detected and
/// which of its TCP ports are being tracked (Settings → Capture).
#[tauri::command]
#[specta::specta]
pub fn get_capture_diagnostics() -> crate::packets::tcp_table::CaptureDiagnostics {
    crate::packets::tcp_table::diagnostics()
}

/// Recent chat messages, optionally filtered by channel id (empty = all).
/// Union (Guild) is channel 4.
#[tauri::command]
#[specta::specta]
pub fn get_chat_messages(
    chat_state: tauri::State<'_, crate::live::chat::ChatStoreMutex>,
    channels: Vec<i32>,
    limit: i64,
) -> Vec<crate::live::chat::ChatRow> {
    let limit = if limit <= 0 { 200 } else { limit.min(500) } as usize;
    chat_state.lock_safe().rows(&channels, limit)
}

/// Clear the in-memory chat log.
#[tauri::command]
#[specta::specta]
pub fn clear_chat(chat_state: tauri::State<'_, crate::live::chat::ChatStoreMutex>) {
    chat_state.lock_safe().clear();
}

/// Enable/disable forwarding Guild (Union) chat to the server-side dedupe relay.
/// The endpoint + API key are baked into the build; this is just the on/off gate.
#[tauri::command]
#[specta::specta]
pub fn set_guild_relay(relay: tauri::State<'_, crate::live::chat::GuildRelayState>, enabled: bool) {
    relay.set(enabled);
    info!(
        "Guild chat relay {}",
        if enabled { "enabled" } else { "disabled" }
    );
}

/// Relay status for the UI: whether the build has the API key configured and
/// whether the dedupe server is reachable (GET /health).
#[tauri::command]
#[specta::specta]
pub async fn get_guild_relay_status() -> crate::live::chat::GuildRelayStatus {
    crate::live::chat::relay_status().await
}
