use futures_util::StreamExt;
pub mod commands;

use std::sync::atomic::Ordering;

use matrix_sdk::{
    authentication::matrix::MatrixSession, config::SyncSettings,
    ruma::events::room::message::SyncRoomMessageEvent, sliding_sync::Version, Client, Room,
    SessionChange,
};
use tauri::{async_runtime::block_on, AppHandle, Emitter, Manager, State};
use tauri_plugin_store::StoreExt;
use url::Url;

use crate::{
    AppData, TimelineWindow, types::auth::{AuthState, error::AuthError}
};

#[cfg(all(debug_assertions))]
const DBG_AUTHPATH: &str = "./debug_secrets/auth.json";

pub async fn handle_sso_callback(
    state: &State<'_, AppData>,
    url: Url,
) -> Result<MatrixSession, AuthError> {
    let client = state
        .client
        .read()
        .await
        .clone()
        .ok_or(AuthError::MissingClient)?;
    let login_builder = client.matrix_auth().login_with_sso_callback(url)?;
    let response = login_builder.await?;
    println!("Login Success! Here is the response: {response:?}");
    let session = client
        .matrix_auth()
        .session()
        .ok_or(AuthError::MissingSession)?;
    Ok(session)
}

pub async fn finish_login(app_handle: AppHandle) {
    let state: State<'_, AppData> = app_handle.state();
    {
        let mut auth_state = state.state.write().await;
        *auth_state = AuthState::Complete;
    }
    {
        let mut torment_client = state.client.write().await;
        let client = torment_client.as_ref().unwrap(); // FIXME: propper errors
        let homeserver_url = client.homeserver();

        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| AuthError::Other(format!("Failed to resolve app data dir: {e}")))
            .unwrap(); // FIXME: propper errors
        let store_dir = app_data_dir
            .join("matrix")
            .join(client.user_id().unwrap().to_string()) // FIXME: propper errors
            .join(client.device_id().unwrap().to_string()); // FIXME: propper errors

        let new_client = Client::builder()
            .homeserver_url(homeserver_url)
            .sqlite_store(&store_dir, None)
            .handle_refresh_tokens()
            .build()
            .await
            .unwrap(); // FIXME: Proper errors
        new_client
            .restore_session(client.session().unwrap()) // FIXME: propper errors
            .await
            .unwrap(); // FIXME: Propper errors

        new_client.add_event_handler(|ev: SyncRoomMessageEvent, room: Room| async move {
            // TODO: put the handler logic in its own rust module
            println!(
                "Received a message {:?} ============== Room {:?} - {:?}",
                ev,
                room.room_id(),
                room.room_type()
            );
        });

        // Persist refreshed session tokens back to auth store so they survive restarts
        let mut session_rx = new_client.subscribe_to_session_changes();
        let session_app_handle = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                match session_rx.recv().await {
                    Ok(SessionChange::TokensRefreshed) => {
                        let state: State<'_, AppData> = session_app_handle.state();
                        let client_guard = state.client.read().await;
                        if let Some(client) = client_guard.as_ref() {
                            if let Some(session) = client.matrix_auth().session() {
                                let store = session_app_handle.store(DBG_AUTHPATH).unwrap();
                                store.set("auth", serde_json::json!(session));
                                store.save().ok();
                                eprintln!("Session tokens refreshed and persisted");
                            }
                        }
                    }
                    Ok(SessionChange::UnknownToken { soft_logout }) => {
                        eprintln!(
                            "Session token invalidated (soft_logout={soft_logout}), \
                             user needs to re-authenticate"
                        );
                        // TODO: emit an event to the frontend to trigger re-login
                    }
                    Err(_) => break, // FIXME: handle this properly // channel closed, client dropped
                }
            }
        });

        *torment_client = Some(new_client);
        println!("Using data directory: {:?}", app_data_dir);
    }
    app_handle.emit("login-success", {}).unwrap(); // FIXME: handle emit errors

    let app_handle = app_handle.clone();
    std::thread::spawn(move || {
        let client = {
            let state: State<'_, AppData> = app_handle.state();
            let guard = state.client.blocking_read();
            guard.clone()
        };
        if let Some(client) = client {
            block_on(async move {
                // TODO: Handle this in its own rust module
                println!("Starting sync");

                // We sync using sync_once at startup so we can let the client know data is ready.
                // If the stored sync token is stale (e.g. server restart, long inactivity,
                // or token refresh desync), we clear the sqlite store and rebuild the client.
                let state: State<'_, AppData> = app_handle.state();
                let client = match client.sync_once(SyncSettings::default()).await {
                    Ok(_) => client,
                    Err(e) if e.to_string().contains("Invalid stream token") => {
                        eprintln!("Stale sync token detected, rebuilding with fresh store...");

                        let homeserver_url = client.homeserver();
                        let session = client.session().expect("client must have a session");
                        let user_id = client
                            .user_id()
                            .expect("client must have a user_id")
                            .to_string();
                        let device_id = client
                            .device_id()
                            .expect("client must have a device_id")
                            .to_string();

                        let app_data_dir = app_handle
                            .path()
                            .app_data_dir()
                            .expect("failed to resolve app data dir");
                        let store_dir = app_data_dir.join("matrix").join(&user_id).join(&device_id);

                        // Drop old client to release sqlite file handles before deleting
                        drop(client);
                        let _ = std::fs::remove_dir_all(&store_dir);

                        let new_client = Client::builder()
                            .homeserver_url(homeserver_url)
                            .sqlite_store(&store_dir, None)
                            .handle_refresh_tokens()
                            .build()
                            .await
                            .expect("failed to rebuild client");

                        new_client
                            .restore_session(session)
                            .await
                            .expect("failed to restore session on rebuilt client");

                        new_client.add_event_handler(
                            |ev: SyncRoomMessageEvent, room: Room| async move {
                                println!(
                                    "Received a message {:?} ============== Room {:?} - {:?}",
                                    ev,
                                    room.room_id(),
                                    room.room_type()
                                );
                            },
                        );

                        // Update shared state so the rest of the app uses the new client
                        {
                            *state.client.write().await = Some(new_client.clone());
                        }

                        new_client
                    }
                    Err(e) => panic!("sync_once failed: {e}"),
                };

                app_handle.emit("sync-ready", {}).unwrap();
                state.has_synced.store(true, Ordering::Relaxed); // TODO: verify if this state will ever need to be set back tto false

                let sliding_sync_builder = client
                    .sliding_sync("main-sync")
                    .map_err(|e| e.to_string())
                    .unwrap()
                    .with_all_extensions()
                    .version(Version::Native);
                let sliding_sync = sliding_sync_builder.build().await.unwrap();
                {
                    *state.sliding.write().await = Some(sliding_sync.clone());
                }
                let mut stream = Box::pin(sliding_sync.sync());
                while let Some(update) = stream.next().await {
                    match update {
                        Ok(summary) => {
                            println!("SUMMARY ====> {summary:?}");

                        }
                        Err(e) => {
                            println!("{}", e.to_string());
                        }
                    }
                }

                println!("Sync stopped");
            });
        } else {
            // FIXME: Remove this
            unreachable!();
        }
    });
}

/// Shared logic for processing an SSO redirect URL, used by both the deep link
/// handler (release builds) and the localhost callback server (debug builds).
pub async fn process_sso_redirect(app_handle: AppHandle, url: Url) -> Result<(), AuthError> {
    let state: State<'_, AppData> = app_handle.state();
    {
        let mut auth_state = state.state.write().await;
        if matches!(&*auth_state, AuthState::InProgress) {
            eprintln!("Redirect encountered while authentication is in progress (non-fatal)");
            return Err(AuthError::InvalidState(AuthState::InProgress));
        } else if !matches!(&*auth_state, AuthState::Initialized) {
            eprintln!("Redirect encountered while not initialized (non-fatal)");
            return Err(AuthError::InvalidState(AuthState::NotStarted));
        } else {
            *auth_state = AuthState::InProgress;
        }
    }
    let session = handle_sso_callback(&state, url).await?;
    // TODO: future feature: save/load the session to long-term storage
    // TODO: REPLACE
    let store = app_handle.store(DBG_AUTHPATH).unwrap();
    store.set("auth", serde_json::json!(session));
    // Save explicitly because the auto-save debounce task may be spawned
    // on a temporary tokio runtime (e.g. the localhost SSO handler thread)
    // that gets dropped before the 100ms debounce fires.
    store.save().ok();

    println!("Session loaded for {}", session.meta.user_id);
    finish_login(app_handle.clone()).await;

    Ok(())
}

/// In debug builds, spin up a one-shot localhost HTTP server to receive the SSO
/// callback instead of relying on deep links (which require a bundled .app on macOS).
#[cfg(all(debug_assertions, target_os = "macos"))]
pub fn spawn_localhost_sso_handler(
    listener: std::net::TcpListener,
    port: u16,
    app_handle: AppHandle,
) {
    std::thread::spawn(move || {
        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(e) => {
                eprintln!("SSO callback server error: {e}");
                return;
            }
        };

        let mut buf = [0u8; 4096];
        let n = match std::io::Read::read(&mut stream, &mut buf) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Failed to read SSO callback request: {e}");
                return;
            }
        };
        let request = String::from_utf8_lossy(&buf[..n]);

        // Parse "GET /?loginToken=xxx HTTP/1.1"
        let path = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/");

        let callback_url = format!("http://localhost:{port}{path}");

        let body = "<html><body><h1>Login successful!</h1><p>You can close this tab and return to the app.</p></body></html>";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body,
        );
        let _ = std::io::Write::write_all(&mut stream, response.as_bytes());
        drop(stream);

        if let Ok(url) = Url::parse(&callback_url) {
            block_on(process_sso_redirect(app_handle, url));
        } else {
            eprintln!("Failed to parse SSO callback URL: {callback_url}");
        }
    });
}
