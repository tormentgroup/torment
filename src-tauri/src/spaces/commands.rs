use futures_util::StreamExt;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::ruma::events::AnyMessageLikeEventContent;
use matrix_sdk_ui::timeline::Timeline;
use matrix_sdk_ui::timeline::TimelineBuilder;
use matrix_sdk_ui::timeline::TimelineItem;
use matrix_sdk_ui::timeline::TimelineItemContent;
use matrix_sdk_ui::timeline::TimelineItemKind;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

use matrix_sdk::deserialized_responses::TimelineEvent;

use matrix_sdk::deserialized_responses::TimelineEventKind;
use matrix_sdk::ruma::events::room::message::MessageType;
use matrix_sdk::ruma::events::AnySyncTimelineEvent;
use matrix_sdk::ruma::UInt;
use matrix_sdk::{
    room::{MessagesOptions, RoomMember},
    ruma::{
        events::{
            room::member::MembershipState, space::child::SpaceChildEventContent,
            AnySyncMessageLikeEvent,
        },
        OwnedRoomId, RoomId,
    },
    RoomMemberships, RoomState,
};
use serde::Serialize;
use tauri::{ipc::IpcResponse, AppHandle, Manager, State};

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
    pub event_id: Option<String>,
    pub sender: String,
    pub ts_ms: i64,

    pub body: String,
    pub formatted_html: Option<String>,
    pub edited: bool,

    pub reactions: BTreeMap<String, u32>,
}

pub fn to_ui_item_from_timeline(it: &Arc<TimelineItem>) -> Option<UiTimelineItem> {
    let event = match it.kind() {
        TimelineItemKind::Event(ev) => ev,
        TimelineItemKind::Virtual(_) => return None,
    };

    let sender = event.sender().to_string();
    let ts_ms: i64 = event.timestamp().0.into();
    let event_id = event.event_id().map(|id| id.to_string());

    if event.content().is_unable_to_decrypt() {
        return Some(UiTimelineItem {
            event_id,
            sender,
            ts_ms,
            body: "Unable to decrypt message".to_string(),
            formatted_html: None,
            edited: false,
            reactions: BTreeMap::new(),
        });
    }

    let msglike = match event.content() {
        TimelineItemContent::MsgLike(msglike) => msglike,
        _ => return None,
    };

    let message = msglike.as_message()?;

    let body = message.body().to_owned();
    let edited = message.is_edited();

    let formatted_html = match message.msgtype() {
        MessageType::Text(text) => text.formatted.as_ref().map(|f| f.body.clone()),
        _ => None,
    };

    let mut reactions: BTreeMap<String, u32> = BTreeMap::new();
    if let Some(r) = event.content().reactions() {
        for (key, by_sender) in r.iter() {
            reactions.insert(key.to_string(), by_sender.len() as u32);
        }
    }

    Some(UiTimelineItem {
        event_id,
        sender,
        ts_ms,
        body,
        formatted_html,
        edited,
        reactions,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn open_room(
    app: tauri::AppHandle,
    room_id: String,
) -> Result<Vec<UiTimelineItem>, String> { // FIXME: add actual error types
    let state: tauri::State<'_, AppData> = app.state();

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

    let chunk: u16 = 100;
    let hard_cap: u32 = 20_000; // TODO: need to stress test and find actual max
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

    let ui: Vec<UiTimelineItem> = items
        .iter()
        .rev()
        .filter_map(to_ui_item_from_timeline)
        .take(target_messages)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

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

    let room_id: OwnedRoomId = RoomId::parse(room_id).map_err(|e| e.to_string())?;

    let client = state.client.read().await.clone().ok_or("missing client")?;
    let room = client.get_room(&room_id).ok_or("not in room")?;

    let timeline = TimelineBuilder::new(&room)
        .build()
        .await
        .map_err(|e| e.to_string())?;

    timeline
        .send(AnyMessageLikeEventContent::RoomMessage(
            RoomMessageEventContent::text_plain(message),
        ))
        .await
        .unwrap();
    Ok(())
}
