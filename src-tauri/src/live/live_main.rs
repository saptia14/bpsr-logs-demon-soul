use crate::live::bptimer_state::{BPTimerEnabledMutex, is_bptimer_enabled};
use crate::live::opcodes_models::EncounterMutex;
use crate::live::opcodes_process::{
    on_server_change, process_aoi_sync_delta, process_sync_container_data,
    process_sync_near_entities, process_sync_to_me_delta_info,
};
use crate::live::player_state::{PlayerCacheMutex, PlayerStateMutex};
use crate::packets;
use crate::protocol::pb;
use crate::utils::sync::MutexExt;
use bytes::Bytes;
use log::{info, warn};
use prost::Message;
use tauri::{AppHandle, Manager};

fn decode_packet<T: Message + Default>(data: Vec<u8>, packet_name: &str) -> Option<T> {
    match T::decode(Bytes::from(data)) {
        Ok(v) => Some(v),
        Err(e) => {
            warn!("Error decoding {packet_name}.. ignoring: {e}");
            None
        }
    }
}

pub async fn start(app_handle: AppHandle) {
    let mut rx = packets::packet_capture::start_capture();

    let bptimer_enabled_state = app_handle.state::<BPTimerEnabledMutex>();

    // 2. Use the channel to receive packets back and process them
    while let Some((op, data)) = rx.recv().await {
        {
            let state = app_handle.state::<EncounterMutex>();
            let encounter = state.lock_safe();
            if encounter.is_encounter_paused {
                continue;
            }
        }
        match op {
            packets::opcodes::Pkt::ServerChangeInfo => {
                let encounter_state = app_handle.state::<EncounterMutex>();
                let mut encounter_state = encounter_state.lock_safe();
                let pending_webhook_state =
                    app_handle.state::<crate::live::webhook::PendingWebhookState>();

                // Disparar reporte automáticamente guardando en caché y pidiendo captura UI
                {
                    let mut pending = pending_webhook_state.lock_safe();
                    *pending = Some(encounter_state.clone());
                }

                if let Err(e) = tauri::Emitter::emit(&app_handle, "request-screenshot", ()) {
                    log::warn!("Failed to request screenshot from frontend: {}", e);
                }

                // Persist the finished encounter to history before it is reset.
                {
                    let db = app_handle.state::<crate::live::database::DatabaseMutex>();
                    let player_cache = app_handle.state::<PlayerCacheMutex>();
                    let player_state = app_handle.state::<PlayerStateMutex>();
                    crate::live::database::save_encounter(
                        &db,
                        &encounter_state,
                        &player_cache.lock_safe(),
                        &player_state.lock_safe(),
                    );
                }

                on_server_change(&mut encounter_state);
            }
            packets::opcodes::Pkt::NotifySocialData => {
                let Some(notify) = decode_packet::<pb::NotifySocialData>(data, "NotifySocialData")
                else {
                    continue;
                };

                let scene_data = notify
                    .v_request
                    .as_ref()
                    .and_then(|r| r.data.as_ref())
                    .and_then(|s| s.scene_data.as_ref());

                if let Some(scene) = scene_data {
                    let player_state_mutex = app_handle.state::<PlayerStateMutex>();
                    let mut player_state = player_state_mutex.lock_safe();

                    let old_line = player_state.get_line_id_opt();
                    if scene.line_id != 0 {
                        player_state.set_line_id(scene.line_id);
                    }
                    if scene.level_map_id != 0 {
                        player_state.set_level_map_id(scene.level_map_id);
                    }

                    if old_line != Some(scene.line_id) && scene.line_id != 0 {
                        info!(
                            "[SocialNtf] scene changed: line_id={} level_map_id={}",
                            scene.line_id, scene.level_map_id
                        );
                        let encounter_state = app_handle.state::<EncounterMutex>();
                        let mut encounter_state = encounter_state.lock_safe();
                        encounter_state.entity_uid_to_entity.clear();
                    }
                }
            }
            packets::opcodes::Pkt::NotifyChatData => {
                let Some(chat) =
                    decode_packet::<pb::NotifyNewestChitChatMsgs>(data, "NotifyNewestChitChatMsgs")
                else {
                    continue;
                };
                let Some(req) = chat.v_request else {
                    continue;
                };
                let Some(chat_msg) = req.chat_msg else {
                    continue;
                };
                let info = chat_msg.msg_info.unwrap_or_default();
                let sender = chat_msg.send_char_info.unwrap_or_default();

                // The game only sends literal text for text/notice messages.
                // Stickers/voice/links carry no text — show a placeholder so the
                // log isn't blank (no media URL is available from the game).
                let is_plain_text = info.msg_type == 0 && !info.msg_text.is_empty();
                let display_text = if !info.msg_text.is_empty() {
                    info.msg_text.clone()
                } else {
                    match info.msg_type {
                        2 => "[notice]",
                        3 => "[sticker]",
                        4 => "[image]",
                        5 => "[voice]",
                        6 => "[link]",
                        _ => "",
                    }
                    .to_string()
                };

                let message = crate::live::chat::ChatMessage {
                    id: 0, // assigned by the store
                    channel: req.channel_type,
                    sender_uid: sender.char_id,
                    sender_name: sender.name.clone(),
                    sender_level: sender.level,
                    msg_type: info.msg_type,
                    text: display_text,
                    timestamp: chat_msg.timestamp,
                    game_msg_id: chat_msg.msg_id,
                };

                info!(
                    "[CHATDEBUG] chat stored: ch={} ({}) sender='{}' lvl={} type={} text='{}'",
                    message.channel,
                    crate::live::chat::channel_name(message.channel),
                    message.sender_name,
                    message.sender_level,
                    message.msg_type,
                    message.text
                );

                // Relay only real Guild (Union) text messages to the dedupe
                // relay (keeps the "Name: text" format clean — no stickers).
                if message.channel == crate::live::chat::CHANNEL_UNION && is_plain_text {
                    let relay = app_handle.state::<crate::live::chat::GuildRelayState>();
                    let key = crate::live::chat::idempotency_key(
                        message.channel,
                        message.game_msg_id,
                        message.sender_uid,
                        message.timestamp,
                    );
                    crate::live::chat::relay_guild_message(
                        &relay,
                        &message.sender_name,
                        &info.msg_text,
                        key,
                    );
                }

                let chat_store = app_handle.state::<crate::live::chat::ChatStoreMutex>();
                chat_store.lock_safe().push(message);
            }
            packets::opcodes::Pkt::SyncNearEntities => {
                let Some(sync_near_entities) =
                    decode_packet::<pb::SyncNearEntities>(data, "SyncNearEntities")
                else {
                    continue;
                };
                let player_state_mutex = app_handle.state::<PlayerStateMutex>();
                let player_state = player_state_mutex.lock_safe();
                let encounter_state = app_handle.state::<EncounterMutex>();
                let mut encounter_state = encounter_state.lock_safe();
                let player_cache_mutex = app_handle.state::<PlayerCacheMutex>();
                if process_sync_near_entities(
                    &mut encounter_state,
                    sync_near_entities,
                    &player_state,
                    is_bptimer_enabled(&bptimer_enabled_state),
                    Some(&player_cache_mutex),
                )
                .is_none()
                {
                    warn!("Error processing SyncNearEntities.. ignoring.");
                }
            }
            packets::opcodes::Pkt::SyncContainerData => {
                let Some(sync_container_data) =
                    decode_packet::<pb::SyncContainerData>(data, "SyncContainerData")
                else {
                    continue;
                };

                // Parse and persist gear modules in their own state (survives
                // the encounter/server-change resets, unlike encounter.local_player).
                {
                    let modules = crate::utils::modules::parse_modules(&sync_container_data);
                    info!("SyncContainerData: parsed {} gear module(s)", modules.len());
                    if !modules.is_empty() {
                        let modules_mutex =
                            app_handle.state::<crate::live::module_optimizer::ModulesMutex>();
                        *modules_mutex.lock_safe() = modules;
                    }
                }

                // Store persistent player identity data
                let mut should_clear_entities = false;
                if let Some(v_data) = &sync_container_data.v_data {
                    let player_state_mutex = app_handle.state::<PlayerStateMutex>();
                    let mut player_state = player_state_mutex.lock_safe();

                    // Extract and store account_id and uid
                    if let Some(char_base) = &v_data.char_base {
                        if !char_base.account_id.is_empty() && v_data.char_id != 0 {
                            player_state
                                .set_account_info(char_base.account_id.clone(), v_data.char_id);
                        }
                    }

                    // Extract and store line_id and level_map_id
                    if let Some(scene_data) = &v_data.scene_data {
                        if scene_data.line_id != 0 {
                            let old_line_id = player_state.get_line_id_opt();
                            player_state.set_line_id(scene_data.line_id);
                            if old_line_id != Some(scene_data.line_id) {
                                should_clear_entities = true;
                            }
                        }
                        if scene_data.level_map_id != 0 {
                            player_state.set_level_map_id(scene_data.level_map_id);
                        }
                    }
                }

                let encounter_state = app_handle.state::<EncounterMutex>();
                let mut encounter_state = encounter_state.lock_safe();
                if should_clear_entities {
                    encounter_state.entity_uid_to_entity.clear();
                }
                let player_cache_mutex = app_handle.state::<PlayerCacheMutex>();
                encounter_state.local_player = Some(sync_container_data.clone());
                if process_sync_container_data(
                    &mut encounter_state,
                    sync_container_data,
                    Some(&player_cache_mutex),
                )
                .is_none()
                {
                    warn!("Error processing SyncContainerData.. ignoring.");
                }
            }
            // packets::opcodes::Pkt::SyncContainerDirtyData => {
            //     // info!("Received {op:?}");
            //     // trace!("Received {op:?} and data {data:?}");
            //     let sync_container_dirty_data =
            //         match blueprotobuf::SyncContainerDirtyData::decode(Bytes::from(data)) {
            //             Ok(v) => v,
            //             Err(e) => {
            //                 warn!("Error decoding SyncContainerDirtyData.. ignoring: {e}");
            //                 continue;
            //             }
            //         };
            //     let encounter_state = app_handle.state::<EncounterMutex>();
            //     let mut encounter_state = encounter_state.lock_safe();
            //     if process_sync_container_dirty_data(&mut encounter_state, sync_container_dirty_data).is_none() {
            //         warn!("Error processing SyncContainerDirtyData.. ignoring.");
            //     }
            // }
            packets::opcodes::Pkt::SyncToMeDeltaInfo => {
                let Some(sync_to_me_delta_info) =
                    decode_packet::<pb::SyncToMeDeltaInfo>(data, "SyncToMeDeltaInfo")
                else {
                    continue;
                };

                let player_state_mutex = app_handle.state::<PlayerStateMutex>();
                let mut player_state = player_state_mutex.lock_safe();

                // Update uid if present in delta_info
                if let Some(delta_info) = &sync_to_me_delta_info.delta_info {
                    let uuid = delta_info.uuid;
                    if uuid != 0 {
                        let local_player_uid =
                            crate::protocol::constants::entity::get_player_uid(uuid);
                        let current_uid = player_state.get_uid_opt();
                        if current_uid != Some(local_player_uid) {
                            player_state.set_uid(local_player_uid);
                        }
                    }
                }

                let encounter_state = app_handle.state::<EncounterMutex>();
                let mut encounter_state = encounter_state.lock_safe();
                let player_cache_mutex = app_handle.state::<PlayerCacheMutex>();
                if process_sync_to_me_delta_info(
                    &mut encounter_state,
                    sync_to_me_delta_info,
                    &player_state,
                    is_bptimer_enabled(&bptimer_enabled_state),
                    Some(&player_cache_mutex),
                )
                .is_none()
                {
                    warn!("Error processing SyncToMeDeltaInfo.. ignoring.");
                }
            }
            packets::opcodes::Pkt::SyncNearDeltaInfo => {
                let Some(sync_near_delta_info) =
                    decode_packet::<pb::SyncNearDeltaInfo>(data, "SyncNearDeltaInfo")
                else {
                    continue;
                };
                let player_state_mutex = app_handle.state::<PlayerStateMutex>();
                let player_state = player_state_mutex.lock_safe();
                let encounter_state = app_handle.state::<EncounterMutex>();
                let mut encounter_state = encounter_state.lock_safe();
                let player_cache_mutex = app_handle.state::<PlayerCacheMutex>();
                for aoi_sync_delta in sync_near_delta_info.delta_infos {
                    if process_aoi_sync_delta(
                        &mut encounter_state,
                        aoi_sync_delta,
                        &player_state,
                        is_bptimer_enabled(&bptimer_enabled_state),
                        Some(&player_cache_mutex),
                    )
                    .is_none()
                    {
                        warn!("Error processing SyncNearDeltaInfo.. ignoring.");
                    }
                }
            }
        }
    }
}
