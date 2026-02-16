use matrix_sdk::{
    authentication::matrix::MatrixSession,
    store::RoomLoadSettings, Client,
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_store::StoreExt;

use crate::{
    auth::DBG_AUTHPATH,
    types::auth::{error::AuthError, AuthState},
    AppData,
};

#[tauri::command(rename_all = "snake_case")]
pub async fn login(app: AppHandle, homeserver_url: String) -> Result<String, AuthError> {
    let state: State<'_, AppData> = app.state();

    // NOTE: Must check state before continuing
    {
        let auth_state = state.state.read().await;
        println!("{auth_state:?}");
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
    }

    let client = Client::builder()
        .homeserver_url(homeserver_url)
        .build()
        .await?;

    // TODO: REMOVE THIS
    let store = app.store(DBG_AUTHPATH).unwrap();
    if let Some(auth) = store.get("auth") {
        eprintln!("Loading auth from file");
        let session: MatrixSession = serde_json::from_value(auth).unwrap();
        client
            .matrix_auth()
            .restore_session(session, RoomLoadSettings::default())
            .await?;
        {
            *(state.client.write().await) = Some(client);
        }
        crate::auth::finish_login(app).await;
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

    // NOTE: Must write state before launching anything
    {
        let mut auth_state = state.state.write().await;
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
        crate::auth::spawn_localhost_sso_handler(listener, port, app.clone());
    }

    Ok(sso_url)
}
