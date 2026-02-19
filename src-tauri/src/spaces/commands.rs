use std::collections::HashMap;

use matrix_sdk::{
    room::RoomMember,
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
        .members(RoomMemberships::all())
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
            MemberInfoMinimal { display_name, avatar_url }
        })
        .collect())
}
