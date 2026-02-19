#![recursion_limit = "256"]
pub mod auth;
pub mod spaces;
pub mod types;

use std::sync::atomic::{AtomicBool, Ordering};

use matrix_sdk::ruma::{events::room::MediaSource, OwnedMxcUri};
use matrix_sdk::{
    media::{MediaFormat, MediaRequestParameters},
    Client,
};
use serde_json::json;
use tauri::{async_runtime::RwLock, AppHandle, Emitter, Manager, State};
use tauri_plugin_deep_link::DeepLinkExt;
use url::Url;

use crate::{
    auth::process_sso_redirect,
    types::auth::{error::AuthError, AuthState},
};

pub struct AppData {
    /// Client needs to be in an Option because it does not get initialized until the user logs in,
    /// and may specify the homeserver_url which means we can't statically get the client at
    /// build time
    client: RwLock<Option<Client>>,
    state: RwLock<AuthState>,
    has_synced: AtomicBool,
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
            spaces::commands::get_rooms,
            spaces::commands::get_spaces,
            spaces::commands::get_members,
            has_synced,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
