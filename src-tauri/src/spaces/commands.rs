use std::collections::HashMap;

use matrix_sdk::{
    ruma::{events::space::child::SpaceChildEventContent, OwnedRoomId, RoomId},
    RoomState,
};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::AppData;

use super::nested_space_ids;

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

        let display_name = child_room
            .display_name()
            .await
            .map(|n| n.to_string())
            .unwrap_or_default();
        let avatar_url = child_room
            .avatar_url()
            .map(|u| u.to_string())
            .unwrap_or_default();
        // If the child is itself a space, count its children from local state too.
        let children_count =
            if child_room.room_type().as_ref().map(|t| t.as_str()) == Some("m.space") {
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

    let all_spaces: Vec<_> = client
        .joined_rooms()
        .into_iter()
        .filter(|r| r.room_type().as_ref().map(|t| t.as_str()) == Some("m.space"))
        .collect();

    let joined_space_ids: std::collections::HashSet<OwnedRoomId> =
        all_spaces.iter().map(|s| s.room_id().to_owned()).collect();

    // Collect all inter-space m.space.child edges with their timestamps.
    let mut edges = Vec::new();
    for space in &all_spaces {
        let Ok(child_events) = space.get_state_events_static::<SpaceChildEventContent>().await
        else {
            continue;
        };
        for raw_event in child_events {
            let Ok(event) = raw_event.deserialize() else {
                continue;
            };
            let child_id = event.state_key().clone();
            if !joined_space_ids.contains(&*child_id) {
                continue;
            }
            edges.push((space.room_id().to_owned(), child_id, event.origin_server_ts()));
        }
    }

    let all_space_ids = all_spaces.iter().map(|s| s.room_id().to_owned());
    let nested = nested_space_ids(edges, all_space_ids);

    let mut result = Vec::new();
    for room in &all_spaces {
        if nested.contains(room.room_id()) {
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
