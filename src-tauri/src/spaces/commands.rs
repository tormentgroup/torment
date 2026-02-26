use futures_util::StreamExt;
use futures_util::TryFutureExt;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::ruma::events::AnyMessageLikeEventContent;
use matrix_sdk_ui::eyeball_im;
use matrix_sdk_ui::timeline::TimelineBuilder;
use matrix_sdk_ui::timeline::TimelineItem;
use matrix_sdk_ui::timeline::TimelineItemContent;
use matrix_sdk_ui::timeline::TimelineItemKind;
use matrix_sdk_ui::Timeline;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;

use matrix_sdk::ruma::events::room::message::MessageType;
use matrix_sdk::{
    ruma::{events::space::child::SpaceChildEventContent, OwnedRoomId, RoomId},
    RoomMemberships, RoomState,
};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use super::graph::{collect_space_edges, SpaceGraph};
use crate::AppData;

#[derive(Serialize)]
pub struct RoomInfoMinimal {
    room_id: String,
    status: RoomState,
    display_name: String,
    avatar_url: String,
    children_count: u64,
    // TODO: Add more fields, we want knowledge about encryption amongst other things
}

#[derive(Serialize)]
pub struct SpaceInfoMinimal {
    room_id: String,
    display_name: String,
    avatar_url: String,
    children_count: u64,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_rooms(app: AppHandle, space_id: String) -> Result<Vec<RoomInfoMinimal>, String> {
    let state: State<'_, AppData> = app.state();
    let client_guard = state.client.read().await;
    let client = client_guard.as_ref().unwrap();

    let id = RoomId::parse(space_id).map_err(|e| e.to_string())?;
    let space = client
        .get_room(&id)
        .ok_or_else(|| format!("Space {id} not found in local state"))?;

    // Build the acyclic space graph so we can filter out cyclic back-references.
    let (_, edges) = collect_space_edges(client).await;
    let graph = SpaceGraph::build(edges);

    // Read m.space.child state events from local store — no server round-trip.
    // Each state key is the room_id of the child room.
    let child_events = space
        .get_state_events_static::<SpaceChildEventContent>()
        .await
        .map_err(|e| e.to_string())?;

    // Index all joined rooms for O(1) child lookups.
    let joined: HashMap<OwnedRoomId, _> = client
        .joined_rooms()
        .into_iter()
        .map(|r| (r.room_id().to_owned(), r))
        .collect();

    let mut result = Vec::new();
    for raw_event in child_events {
        let event = match raw_event.deserialize() {
            Ok(e) => e,
            Err(_) => continue,
        };

        let child_room_id = event.state_key();

        // Skip rooms we haven't joined locally; they'll appear once sync catches up.
        let Some(child_room) = joined.get(child_room_id) else {
            continue;
        };

        let is_space = child_room.room_type().as_ref().map(|t| t.as_str()) == Some("m.space");

        // If the child is a space, only include it if the DAG considers it a valid
        // (non-cyclic) child of this parent. Regular rooms can't form cycles.
        if is_space && !graph.is_valid_child(&id, child_room_id) {
            continue;
        }

        let display_name = child_room
            .display_name()
            .await
            .map(|n| n.to_string())
            .unwrap_or_default();
        let avatar_url = child_room
            .avatar_url()
            .map(|u| u.to_string())
            .unwrap_or_default();
        let children_count = if is_space {
            child_room
                .get_state_events_static::<SpaceChildEventContent>()
                .await
                .map(|evs| evs.len() as u64)
                .unwrap_or(0)
        } else {
            0
        };

        result.push(RoomInfoMinimal {
            room_id: child_room.room_id().to_string(),
            status: RoomState::Joined,
            display_name,
            avatar_url,
            children_count,
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn get_spaces(app: AppHandle) -> Result<Vec<SpaceInfoMinimal>, String> {
    let state: State<'_, AppData> = app.state();
    let client_guard = state.client.read().await;
    let client = client_guard.as_ref().ok_or("Client not ready")?;

    let (all_spaces, edges) = collect_space_edges(client).await;
    let graph = SpaceGraph::build(edges);

    let mut result = Vec::new();
    for room in &all_spaces {
        if graph.nested_ids().contains(room.room_id()) {
            continue;
        }
        let display_name = room
            .display_name()
            .await
            .map(|n| n.to_string())
            .unwrap_or_default();
        let avatar_url = room.avatar_url().map(|u| u.to_string()).unwrap_or_default();
        let children_count = room
            .get_state_events_static::<SpaceChildEventContent>()
            .await
            .map(|evs| evs.len() as u64)
            .unwrap_or(0);

        result.push(SpaceInfoMinimal {
            room_id: room.room_id().to_string(),
            display_name,
            avatar_url,
            children_count,
        });
    }
    Ok(result)
}

#[derive(Serialize)]
pub struct MemberInfoMinimal {
    display_name: String,
    avatar_url: String,
    id: String,
}

// FIXME: handle errors
#[tauri::command]
pub async fn get_members(
    app: AppHandle,
    room_id: String,
) -> Result<Vec<MemberInfoMinimal>, String> {
    let state: State<'_, AppData> = app.state();
    let client_guard = state.client.read().await;
    let client = client_guard.as_ref().ok_or("Client not ready")?;
    let room = client.get_room(&RoomId::parse(room_id).unwrap()).unwrap();
    let members = room
        .members(RoomMemberships::JOIN)
        .await
        .map_err(|e| e.to_string())?;
    Ok(members
        .iter()
        .map(|mem| {
            let display_name = match mem.display_name() {
                Some(name) => name.to_string(),
                _ => mem.name().to_string(),
            };
            let avatar_url = match mem.avatar_url() {
                Some(url) => url.to_string(),
                _ => "".to_string(),
            };
            MemberInfoMinimal {
                display_name,
                avatar_url,
                id: mem.user_id().to_string(),
            }
        })
        .collect())
}

#[derive(Debug, Clone, Serialize)]
pub struct UiTimelineItem {
    pub key: String,
    pub event_id: Option<String>,
    pub sender: String,
    pub ts_ms: i64,

    pub body: String,
    pub formatted_html: Option<String>,
    pub edited: bool,
    pub reactions: BTreeMap<String, u32>,

    pub is_redacted: bool, // NEW
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiRow {
    Message {
        key: String,
        message: UiTimelineItem,
    },
    DateDivider {
        key: String,
        ts_ms: i64,
    },
    TimelineStart {
        key: String,
    },
    Other {
        key: String,
    },
}
pub fn to_ui_row(it: &Arc<TimelineItem>) -> UiRow {
    let key = it.unique_id().0.clone();
    match it.kind() {
        TimelineItemKind::Virtual(v) => {
            use matrix_sdk_ui::timeline::VirtualTimelineItem;

            match v {
                VirtualTimelineItem::DateDivider(ts) => {
                    // ts is a MilliSecondsSinceUnixEpoch in many versions; adapt as needed
                    let ts_ms: i64 = ts.0.into();
                    UiRow::DateDivider { key, ts_ms }
                }
                VirtualTimelineItem::TimelineStart => UiRow::TimelineStart { key },
                _ => UiRow::Other { key }, // keep it future-proof for other virtual variants
            }
        }

        TimelineItemKind::Event(_) => {
            // reuse your existing converter for messages
            if let Some(msg) = to_ui_item_from_timeline(it) {
                UiRow::Message { message: msg, key }
            } else {
                UiRow::Other { key }
            }
        }
    }
}
pub fn to_ui_item_from_timeline(it: &Arc<TimelineItem>) -> Option<UiTimelineItem> {
    let event = match it.kind() {
        TimelineItemKind::Event(ev) => ev,
        TimelineItemKind::Virtual(_) => return None,
    };

    let sender = event.sender().to_string();
    let ts_ms: i64 = event.timestamp().0.into();
    let event_id = event.event_id().map(|id| id.to_string());
    let key = it.unique_id().0.clone();
    // If it can't decrypt, still show placeholder.
    if event.content().is_unable_to_decrypt() {
        return Some(UiTimelineItem {
            key,
            event_id,
            sender,
            ts_ms,
            body: "Unable to decrypt message".to_string(),
            formatted_html: None,
            edited: false,
            reactions: BTreeMap::new(),
            is_redacted: false,
        });
    }

    // Keep reactions regardless of content kind
    let mut reactions: BTreeMap<String, u32> = BTreeMap::new();
    if let Some(r) = event.content().reactions() {
        for (key, by_sender) in r.iter() {
            reactions.insert(key.to_string(), by_sender.len() as u32);
        }
    }

    // We only render message-like items; BUT if it was redacted, the SDK may no longer
    // expose it as a message. In that case, return a placeholder instead of None.
    let msglike = match event.content() {
        TimelineItemContent::MsgLike(msglike) => msglike,
        _ => {
            // For now, ignore other event types.
            return None;
        }
    };

    if let Some(message) = msglike.as_message() {
        let body = message.body().to_owned();
        let edited = message.is_edited();
        let formatted_html = match message.msgtype() {
            MessageType::Text(text) => text.formatted.as_ref().map(|f| f.body.clone()),
            _ => None,
        };

        return Some(UiTimelineItem {
            key,
            event_id,
            sender,
            ts_ms,
            body,
            formatted_html,
            edited,
            reactions,
            is_redacted: false,
        });
    }

    // If it was message-like but no longer a message (common after redaction),
    // emit a stable placeholder so the UI can update.
    // TODO: Please one day verify what the AI overlord assumes
    Some(UiTimelineItem {
        key,
        event_id,
        sender,
        ts_ms,
        body: "Message deleted".to_string(),
        formatted_html: None,
        edited: false,
        reactions,
        is_redacted: true,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum UiTimelinePatch {
    PushBack { row: UiRow },
    PushFront { row: UiRow },
    Insert { index: usize, row: UiRow },
    Set { index: usize, row: UiRow },
    Remove { index: usize },
}

#[tauri::command(rename_all = "snake_case")]
pub async fn open_room(app: tauri::AppHandle, room_id: String) -> Result<Vec<UiRow>, String> {
    // FIXME: add actual error types
    let state: tauri::State<'_, AppData> = app.state();

    {
        let timeline_event_task = state.timeline_event_task.write().await;
        if let Some(task) = &*timeline_event_task {
            task.abort();
        }
    }

    let room_id: OwnedRoomId = RoomId::parse(room_id).map_err(|e| e.to_string())?;

    let client = state.client.read().await.clone().ok_or("missing client")?;
    let room = client.get_room(&room_id).ok_or("not in room")?;

    let timeline = TimelineBuilder::new(&room)
        .build()
        .await
        .map_err(|e| e.to_string())?;

    let target_messages = 20usize;

    let (items_snapshot, mut diffs_stream) = timeline.subscribe().await;
    let mut items = items_snapshot;

    let chunk: u16 = 20u16;
    let hard_cap: u32 = 20_000u32; // TODO: need to stress test and find actual max
    let mut total_requested: u32 = 0;

    loop {
        let renderable = items
            .iter()
            .filter(|it| to_ui_item_from_timeline(it).is_some())
            .count();
        if renderable >= target_messages || total_requested >= hard_cap {
            break;
        }

        let reached_start = timeline
            .paginate_backwards(chunk)
            .await
            .map_err(|e| e.to_string())?;
        total_requested += chunk as u32;

        while let Some(diffs) = diffs_stream.next().await {
            let mut any = false;
            for diff in diffs {
                diff.apply(&mut items);
                any = true;
            }

            if any {
                let renderable_now = items
                    .iter()
                    .filter(|it| to_ui_item_from_timeline(it).is_some())
                    .count();
                if renderable_now >= target_messages {
                    break;
                }

                break;
            }
        }

        if reached_start {
            break;
        }
    }

    {
        let mut open_timeline = state.open_timeline.write().await;
        *open_timeline = Some(timeline);
    }
    let app_handle = app.clone();
    let room_id_str = room_id.to_string();

    let task = tauri::async_runtime::spawn(async move {
        while let Some(diffs) = diffs_stream.next().await {
            let mut patches: Vec<UiTimelinePatch> = Vec::new();

            for diff in diffs {
                match diff {
                    eyeball_im::VectorDiff::PushBack { value } => {
                        let row = to_ui_row(&value);
                        patches.push(UiTimelinePatch::PushBack { row });
                    }
                    eyeball_im::VectorDiff::PushFront { value } => {
                        let row = to_ui_row(&value);
                        patches.push(UiTimelinePatch::PushFront { row });
                    }
                    eyeball_im::VectorDiff::Insert { index, value } => {
                        let row = to_ui_row(&value);
                        patches.push(UiTimelinePatch::Insert { index, row });
                    }
                    eyeball_im::VectorDiff::Set { index, value } => {
                        let row = to_ui_row(&value);
                        patches.push(UiTimelinePatch::Set { index, row });
                    }
                    eyeball_im::VectorDiff::Remove { index } => {
                        patches.push(UiTimelinePatch::Remove { index });
                    }
                    eyeball_im::VectorDiff::Append { .. } => {
                        todo!()
                    }
                    eyeball_im::VectorDiff::Reset { .. } => {
                        todo!()
                    }
                    eyeball_im::VectorDiff::Clear { .. } => {
                        todo!()
                    }
                    eyeball_im::VectorDiff::PopBack { .. } => {
                        todo!()
                    }
                    eyeball_im::VectorDiff::Truncate { .. } => {
                        todo!()
                    }
                    eyeball_im::VectorDiff::PopFront { .. } => {
                        todo!()
                    }
                }
            }

            if !patches.is_empty() {
                let _ = app_handle.emit(
                    "timeline-patch",
                    serde_json::json!({
                        "room_id": room_id_str,
                        "patches": patches,
                    }),
                );
            }
        }
    });
    {
        let mut timeline_event_task = state.timeline_event_task.write().await;
        *timeline_event_task = Some(task);
    }
    let ui: Vec<UiRow> = items.iter().map(to_ui_row).collect();

    Ok(ui)
}

#[tauri::command(rename_all = "snake_case")]
// FIXME: Add real error handling
pub async fn send_message(
    app: tauri::AppHandle,
    room_id: String,
    message: String,
) -> Result<(), String> {
    let state: tauri::State<'_, AppData> = app.state();

    let timeline = state.open_timeline.write().await;
    if let Some(timeline) = &*timeline {
        timeline
            .send(AnyMessageLikeEventContent::RoomMessage(
                RoomMessageEventContent::text_plain(message),
            ))
            .await
            .unwrap();
        Ok(())
    } else {
        Err("No timeline exists for the open room".to_string())
    }
}

#[tauri::command(rename_all = "snake_case")]
// FIXME: Add real error handling
pub async fn paginate_up(app: tauri::AppHandle) -> Result<(), String> {
    let state: State<'_, AppData> = app.state();
    let timeline = state.open_timeline.write().await;
    if let Some(timeline) = &*timeline {
        let _at_start = timeline
            .paginate_backwards(10)
            .await
            .map_err(|e| e.to_string())?; // TODO: possibly emit an event that we have reached the start/end of the timeline
        Ok(())
    } else {
        Err("No open timeline found".to_string())
    }
}
#[tauri::command(rename_all = "snake_case")]
// FIXME: Add real error handling
pub async fn paginate_down(app: tauri::AppHandle) -> Result<(), String> {
    let state: State<'_, AppData> = app.state();
    let timeline = state.open_timeline.write().await;
    if let Some(timeline) = &*timeline {
        let _at_start = timeline
            .paginate_forwards(10)
            .await
            .map_err(|e| e.to_string())?; // TODO: possibly emit an event that we have reached the start/end of the timeline
        Ok(())
    } else {
        Err("No open timeline found".to_string())
    }
}
