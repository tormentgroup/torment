#![recursion_limit = "256"]
pub mod auth;
pub mod types;

use std::sync::atomic::{AtomicBool, Ordering};

use matrix_sdk::ruma::RoomId;
use matrix_sdk::ruma::{events::room::MediaSource, OwnedMxcUri};
use matrix_sdk::stream::StreamExt;
use matrix_sdk::{
    media::{MediaFormat, MediaRequestParameters},
    Client, RoomState,
};
use matrix_sdk_ui::spaces::room_list::SpaceRoomListPaginationState;
use matrix_sdk_ui::spaces::{SpaceRoom, SpaceService};
use serde::Serialize;
use serde_json::json;
use tauri::{async_runtime::RwLock, AppHandle, Emitter, Manager, State};
use tauri_plugin_deep_link::DeepLinkExt;
use url::Url;

use crate::{
    auth::process_sso_redirect,
    types::auth::{error::AuthError, AuthState},
};

#[derive(Serialize)]
pub struct RoomInfoMinimal {
    room_id: String,
    status: RoomState,
    display_name: String,
    avatar_url: String,
    children_count: u64,
    // TODO: Add more fields, we want knowledge about encryption amongst other things
}

#[tauri::command(rename_all = "snake_case")]
// FIXME: Need to handle errors
async fn get_rooms(app: AppHandle, space_id: String) -> Result<Vec<RoomInfoMinimal>, String> {
    let state: State<'_, AppData> = app.state();
    let client = state.client.read().await;
    let client = client.as_ref().unwrap();
    let space_service = SpaceService::new(client.clone());
    let id = RoomId::parse(space_id).map_err(|e|e.to_string())?;
    let room_list = space_service
        .space_room_list(id)
        .await;

    room_list
        .paginate()
        .await
        .map_err(|e| e.to_string())
        .unwrap();
    loop {
        match room_list.pagination_state() {
            SpaceRoomListPaginationState::Idle { end_reached: true } => break,
            SpaceRoomListPaginationState::Idle { end_reached: false } => {
                room_list
                    .paginate()
                    .await
                    .map_err(|e| e.to_string())
                    .unwrap();
            }
            _ => {
                let mut s = room_list.subscribe_to_pagination_state_updates();
                let _ = s.next().await;
            }
        }
    }

    let mut result = Vec::new();
    let rooms = room_list.rooms()
        .into_iter()
        .filter(|v| v.state == Some(RoomState::Joined))
        .collect::<Vec<SpaceRoom>>();
    for room in rooms {
        let avatar_url = if let Some(url) = room.avatar_url {
            url.to_string()
        } else {
            "".to_string()
        };
        result.push(RoomInfoMinimal {
            room_id: room.room_id.to_string(),
            status: room.state.unwrap_or(RoomState::Joined),
            display_name: room.display_name,
            children_count: room.children_count,
            avatar_url,
        });
    }
    Ok(result)
}

#[derive(Serialize)]
pub struct SpaceInfoMinimal {
    room_id: String,
    display_name: String,
    avatar_url: String,
    children_count: u64,
}
#[tauri::command]
async fn get_spaces(app: tauri::AppHandle) -> Result<Vec<SpaceInfoMinimal>, String> {
    let state: tauri::State<'_, AppData> = app.state();
    let client_guard = state.client.read().await;
    let client = client_guard.as_ref().ok_or("Client not ready")?.clone();

    let space_service = SpaceService::new(client);
    let spaces = space_service.joined_spaces().await;

    let result = spaces
        .into_iter()
        .map(|s| SpaceInfoMinimal {
            room_id: s.room_id.to_string(),
            display_name: s.display_name,
            avatar_url: s.avatar_url.map(|mxc| mxc.to_string()).unwrap_or_default(),
            children_count: s.children_count,
        })
        .collect();

    Ok(result)
}

#[tauri::command]
async fn has_synced(app: AppHandle) -> bool {
    let state: State<'_, AppData> = app.state();
    state.has_synced.load(Ordering::Relaxed)
}

fn extract_url(urls: &[Url]) -> Option<Url> {
    urls.iter()
        .find(|url| url.scheme() == "torment" && url.host_str() == Some("auth"))
        .cloned()
}

pub struct AppData {
    /// Client needs to be in an Option because it does not get initialized until the user logs in,
    /// and may specify the homeserver_url which means we can't statically get the client at
    /// build time
    client: RwLock<Option<Client>>,
    state: RwLock<AuthState>,
    has_synced: AtomicBool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppData {
            client: RwLock::new(None),
            state: RwLock::new(AuthState::NotStarted),
            has_synced: AtomicBool::new(false),
        })
        .register_asynchronous_uri_scheme_protocol("mxc", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                let uri = request.uri();
                // Reconstruct mxc:// URI from the request URL
                // Tauri gives us: mxc://localhost/server_name/media_id
                // We need: mxc://server_name/media_id
                let path = uri.path();
                let host = uri.host().unwrap_or_default();
                let mxc_uri: OwnedMxcUri = format!("mxc://{}{}", host, path).into();

                let state: State<'_, AppData> = app.state();
                let client = state.client.read().await;
                let Some(client) = client.as_ref() else {
                    responder.respond(
                        tauri::http::Response::builder()
                            .status(503)
                            .body(b"Client not ready".to_vec())
                            .unwrap(),
                    );
                    return;
                };

                let request = MediaRequestParameters {
                    source: MediaSource::Plain(mxc_uri),
                    format: MediaFormat::File,
                };

                match client.media().get_media_content(&request, true).await {
                    Ok(data) => {
                        responder.respond(
                            tauri::http::Response::builder()
                                .status(200)
                                .header("Content-Type", "application/octet-stream")
                                .body(data)
                                .unwrap(),
                        );
                    }
                    Err(e) => {
                        eprintln!("Failed to fetch media: {e}");
                        responder.respond(
                            tauri::http::Response::builder()
                                .status(404)
                                .body(format!("Media not found: {e}").into_bytes())
                                .unwrap(),
                        );
                    }
                }
            });
        })
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            let window = app.get_webview_window("torment");
            if let Some(window) = window {
                _ = window.set_focus();
            } else {
                eprintln!("Could not find window: torment");
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            // NOTE: that get_current's return value will also get updated every time on_open_url gets triggered.
            //let start_urls = app.deep_link().get_current()?;
            //if let Some(urls) = start_urls {
            //    // app was likely started by a deep link
            //    println!("deep link URLs: {:?}", urls);
            //}
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                app.deep_link().register_all()?;
            }

            let app_handle = app.handle();
            app.deep_link().on_open_url({
                let app_handle_outer = app_handle.clone();
                move |event| {
                    let urls = event.urls();
                    let url = extract_url(&urls);
                    println!("Your redirect url: {url:?}");
                    let app_handle = app_handle_outer.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(url) = url {
                            match process_sso_redirect(app_handle.clone(), url).await {
                                Ok(_) => {}
                                Err(e) => {
                                    app_handle.emit("login-error", json!(e)).unwrap();
                                    // FIXME: handle emit errors
                                }
                            };
                        } else {
                            // FIXME: add proper error handling
                            // TODO: emit an error event to the frontend
                            eprintln!("No token found on redirect");
                            let state: State<'_, AppData> = app_handle.state();
                            let mut auth_state = state.state.write().await;
                            app_handle
                                .emit(
                                    "login-error",
                                    AuthError::Other("No token found on redirect".to_string()),
                                )
                                .unwrap(); //FIXME: Need to think about how emit errors are handled
                            *auth_state =
                                AuthState::Failed("No token found on redirect".to_string());
                        }
                    });
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth::commands::login,
            get_rooms,
            get_spaces,
            has_synced,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
