#![recursion_limit = "256"]
use matrix_sdk::{
    Client, ClientBuildError, Room, authentication::matrix::MatrixSession, config::SyncSettings, ruma::events::room::message::SyncRoomMessageEvent
};
use serde::Serialize;
use tauri::{
    async_runtime::{block_on, RwLock},
    Manager, State,
};
use tauri_plugin_deep_link::DeepLinkExt;
use thiserror::Error;
use url::Url;

#[tauri::command(rename_all = "snake_case")]
async fn login(state: State<'_, AppData>, homeserver_url: String) -> Result<String, AuthError> {
    let client = Client::builder()
        .homeserver_url(homeserver_url)
        .build()
    .await?;
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
        client.add_event_handler(|ev: SyncRoomMessageEvent, room: Room,| async move {
            // TODO: put the handler logic in its own rust module
            println!("Received a message {:?} ============== Room {:?} - {:?}", ev, room.room_id(), room.room_type());
        });
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
    Ok(sso_url)
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
                        let state: State<'_, AppData> = app_handle.state();
                        if let Some(url) = url {
                            {
                                let mut auth_state = state.state.write().await;
                                if matches!(&*auth_state, AuthState::InProgress) {
                                    eprintln!(
                                    "Redirect encountered while authentication is in progress (non-fatal)"
                                );
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
                                    println!("Session loaded for {}", session.meta.user_id);
                                    {
                                        let mut auth_state = state.state.write().await;
                                        *auth_state = AuthState::Complete;
                                    }
                                    // TODO: emit login success event to the frontend

                                    let app_handle = app_handle.clone();
                                    std::thread::spawn(move || {
                                        let state: State<'_, AppData> = app_handle.state();
                                        let client = state.client.blocking_write();
                                        if let Some(client) = &*client {
                                            let client = client.clone();
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
                                Err(e) => {
                                    // FIXME: add proper error handling
                                    // - occurs when authentication fails
                                    // TODO: emit an error event to the frontend
                                    eprintln!("{e}");
                                    {
                                        let mut auth_state = state.state.write().await;
                                        *auth_state = AuthState::Failed(e.to_string());
                                    }
                                }
                            };
                        } else {
                            // FIXME: add proper error handling
                            // - occurs when there was no url with sso query from the redirect
                            // TODO: emit an error event to the frontend
                            eprintln!("No token found on redirect");
                            {
                                let mut auth_state = state.state.write().await;
                                *auth_state =
                                    AuthState::Failed("No token found on redirect".to_string());
                            }
                            return;
                        }
                    });
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![login])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
