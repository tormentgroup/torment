#![recursion_limit = "256"]
use matrix_sdk::{
    Client, ClientBuildError, Room, authentication::matrix::MatrixSession, config::SyncSettings, ruma::{events::room::message::SyncRoomMessageEvent, room::RoomType}, store::RoomLoadSettings
};
use serde::Serialize;
use tauri::{
    AppHandle, Manager, State, async_runtime::{RwLock, block_on}
};
use tauri_plugin_store::StoreExt;
use tauri_plugin_deep_link::DeepLinkExt;
use thiserror::Error;
use url::Url;


const DBG_AUTHPATH: &str = "./debug_secrets/auth.json";


#[tauri::command(rename_all = "snake_case")]
async fn login(app: AppHandle, homeserver_url: String) -> Result<String, AuthError> {
    let state: State<'_, AppData> = app.state();
    let client = Client::builder()
        .homeserver_url(homeserver_url)
        .build()
    .await?;
    client.add_event_handler(|ev: SyncRoomMessageEvent, room: Room,| async move {
        // TODO: put the handler logic in its own rust module
        println!("Received a message {:?} ============== Room {:?} - {:?}", ev, room.room_id(), room.room_type());
    });

    // TODO: REMOVE THIS
    let store = app.store(DBG_AUTHPATH).unwrap();
    if let Some(auth) = store.get("auth") {
        eprintln!("Loading auth from file");
        let session: MatrixSession = serde_json::from_value(auth).unwrap();
        client.matrix_auth().restore_session(session, RoomLoadSettings::default()).await?;
        {
            *(state.client.write().await) = Some(client);
        }
        finish_login(app).await;
        return Ok("".to_string());
    }

    // In debug builds on macOS, use a localhost callback server instead of deep links
    // because macOS requires a bundled .app for custom URL schemes to work.
    // Linux and Windows can register deep links at runtime via register_all().
    #[cfg(all(debug_assertions, target_os = "macos"))]
    let (sso_url, sso_listener) = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| AuthError::Other(format!("Failed to bind SSO callback server: {e}")))?;
        let port = listener.local_addr().unwrap().port();
        println!("SSO callback server listening on http://localhost:{port}");
        let url = client
            .matrix_auth()
            .get_sso_login_url(&format!("http://localhost:{port}"), None)
        .await?;
        (url, (listener, port))
    };
    #[cfg(not(all(debug_assertions, target_os = "macos")))]
    let sso_url = client
        .matrix_auth()
        .get_sso_login_url("torment://auth", None)
    .await?;

    {
        let mut auth_state = state.state.write().await;
        match &*auth_state {
            AuthState::InProgress => {
                eprintln!("Login attempted while authentication is in progress.");
                return Err(AuthError::InvalidState(AuthState::InProgress));
            }
            AuthState::Complete => {
                eprintln!("Login attempted when already authenticated.");
                return Err(AuthError::InvalidState(AuthState::Complete));
            }
            AuthState::Failed(_) | AuthState::NotStarted | AuthState::Initialized => {}
        }
        *auth_state = AuthState::Initialized;
    }
    {
        let mut matrix_client = state.client.write().await;
        *matrix_client = Some(client);
    }
    // NOTE: must be last because both the state and client have to be set in order for callback to work
    match open::that(sso_url.clone()) {
        Ok(_) => {}
        Err(e) => {
            let mut auth_state = state.state.write().await;
            *auth_state = AuthState::Failed(e.to_string());
            return Err(AuthError::Other(e.to_string()));
        }
    }

    #[cfg(all(debug_assertions, target_os = "macos"))]
    {
        let (listener, port) = sso_listener;
        spawn_localhost_sso_handler(listener, port, app.clone());
    }

    Ok(sso_url)
}


#[tauri::command]
async fn get_rooms(app: AppHandle) -> Vec<matrix_sdk::RoomInfo> {
    let state: State<'_, AppData> = app.state();
    let client = state.client.read().await;
    let client = client.as_ref().unwrap();
    let mut result = Vec::new();
    for room in client.rooms() {
        result.push(room.clone_info());
    }
    result
}


#[derive(Error, Debug, Serialize)]
#[serde(tag = "type")]
pub enum AuthError {
    #[error("SSO callback received but no Matrix client is stored")]
    MissingClient,
    #[error("SSO callback URL was not a valid Matrix login callback: {0}")]
    Sso(String),
    #[error("Matrix Error during SSO login: {0}")]
    Matrix(String),
    #[error("Login succeeded but SDK returned no session (unexpected)")]
    MissingSession,
    #[error("Failed to construct the client. Is the homeserver_url correct? {0}")]
    ClientBuilder(String),
    #[error("Invalid auth state: {0:?}")]
    InvalidState(AuthState),
    #[error("{0}")]
    Other(String),
}

impl From<matrix_sdk::authentication::matrix::SsoError> for AuthError {
    fn from(e: matrix_sdk::authentication::matrix::SsoError) -> Self {
        AuthError::Sso(e.to_string())
    }
}

impl From<matrix_sdk::Error> for AuthError {
    fn from(e: matrix_sdk::Error) -> Self {
        AuthError::Matrix(e.to_string())
    }
}

impl From<ClientBuildError> for AuthError {
    fn from(e: ClientBuildError) -> Self {
        AuthError::ClientBuilder(e.to_string())
    }
}

fn extract_url(urls: &[Url]) -> Option<Url> {
    urls.iter()
        .find(|url| url.scheme() == "torment" && url.host_str() == Some("auth"))
        .cloned()
}

async fn handle_sso_callback(
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


async fn finish_login(app_handle: AppHandle) {
    let state: State<'_, AppData> = app_handle.state();
    {
        let mut auth_state = state.state.write().await;
        *auth_state = AuthState::Complete;
    }
    // TODO: emit login success event to the frontend

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
async fn process_sso_redirect(app_handle: AppHandle, url: Url) {
    let state: State<'_, AppData> = app_handle.state();
    {
        let mut auth_state = state.state.write().await;
        if matches!(&*auth_state, AuthState::InProgress) {
            eprintln!("Redirect encountered while authentication is in progress (non-fatal)");
            return;
        } else if !matches!(&*auth_state, AuthState::Initialized) {
            eprintln!("Redirect encountered while not initialized (non-fatal)");
            return;
        } else {
            *auth_state = AuthState::InProgress;
        }
    }
    match handle_sso_callback(&state, url).await {
        Ok(session) => {
            // TODO: future feature: save/load the session to long-term storage
            // TODO: REPLACE
            let store = app_handle.store(DBG_AUTHPATH).unwrap();
            store.set("auth", serde_json::json!(session));
            // Save explicitly because the auto-save debounce task may be spawned
            // on a temporary tokio runtime (e.g. the localhost SSO handler thread)
            // that gets dropped before the 100ms debounce fires.
            store.save().ok();

            println!("Session loaded for {}", session.meta.user_id);
            finish_login(app_handle).await;
        }
        Err(e) => {
            // FIXME: add proper error handling
            // TODO: emit an error event to the frontend
            eprintln!("{e}");
            {
                let mut auth_state = state.state.write().await;
                *auth_state = AuthState::Failed(e.to_string());
            }
        }
    }
}

/// In debug builds, spin up a one-shot localhost HTTP server to receive the SSO
/// callback instead of relying on deep links (which require a bundled .app on macOS).
#[cfg(all(debug_assertions, target_os = "macos"))]
fn spawn_localhost_sso_handler(listener: std::net::TcpListener, port: u16, app_handle: AppHandle) {
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

/// AuthState begins in NotStarted. After launching the sso url on the user's browser, AuthState is
/// Initialized. When a redirect is encountered, AuthState moves to InProgress and stays there
/// until either an error occurs, or authentication succeeds.
/// NOTE: This is our state machine which protects against race conditions.
#[derive(Debug, Serialize, Clone)]
pub enum AuthState {
    /// Initial state
    NotStarted,
    /// gets set after launching SSO Url
    Initialized,
    /// gets set only when an SSO Url was found on a redirect
    InProgress,
    /// gets set only when the user completes authentication with no errors
    Complete,
    /// gets set only when the user fails authentication to any variety of reasons
    Failed(String),
}

pub struct AppData {
    /// Client needs to be in an Option because it does not get initialized until the user logs in,
    /// and may specify the homeserver_url which means we can't statically get the client at
    /// build time
    client: RwLock<Option<Client>>,
    state: RwLock<AuthState>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppData {
            client: RwLock::new(None),
            state: RwLock::new(AuthState::NotStarted),
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
                            process_sso_redirect(app_handle, url).await;
                        } else {
                            // FIXME: add proper error handling
                            // TODO: emit an error event to the frontend
                            eprintln!("No token found on redirect");
                            let state: State<'_, AppData> = app_handle.state();
                            let mut auth_state = state.state.write().await;
                            *auth_state =
                                AuthState::Failed("No token found on redirect".to_string());
                        }
                    });
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            login,
            get_rooms,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
