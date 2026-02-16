pub mod commands;

use matrix_sdk::{
    authentication::matrix::MatrixSession, config::SyncSettings,
    ruma::events::room::message::SyncRoomMessageEvent, Client, Room,
};
use tauri::{async_runtime::block_on, AppHandle, Emitter, Manager, State};
use tauri_plugin_store::StoreExt;
use url::Url;

use crate::{
    types::auth::{error::AuthError, AuthState},
    AppData,
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
            .sqlite_store(store_dir, None)
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
        *torment_client = Some(new_client);
        println!("Using data directory: {:?}", app_data_dir);
    }
    // TODO: emit login success event to the frontend
    app_handle.emit("login-success", {}).unwrap(); // FIXME: handle emit errors

    let app_handle = app_handle.clone();
    std::thread::spawn(move || {
        let state: State<'_, AppData> = app_handle.state();
        let client = {
            let guard = state.client.blocking_read();
            guard.clone()
        };
        if let Some(client) = client {
            block_on(async move {
                // TODO: Handle this in its own rust module
                println!("Starting sync");
                client.sync(SyncSettings::default()).await.unwrap();
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
