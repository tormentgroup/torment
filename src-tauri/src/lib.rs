#![recursion_limit = "256"]
pub mod auth;
pub mod types;

use matrix_sdk::{
    Client,
};
use tauri::{
    AppHandle, Emitter, Manager, State, async_runtime::RwLock
};
use tauri_plugin_deep_link::DeepLinkExt;
use url::Url;

use crate::{auth::process_sso_redirect, types::auth::{AuthState, error::AuthError}};


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
                            match process_sso_redirect(app_handle.clone(), url).await {
                                Ok(_) => {}
                                Err(e) => {
                                    app_handle.emit("login-error", e).unwrap(); // FIXME: handle emit errors
                                }
                            };
                        } else {
                            // FIXME: add proper error handling
                            // TODO: emit an error event to the frontend
                            eprintln!("No token found on redirect");
                            let state: State<'_, AppData> = app_handle.state();
                            let mut auth_state = state.state.write().await;
                            app_handle.emit("login-error", AuthError::Other("No token found on redirect".to_string())).unwrap(); //FIXME: Need to think about how emit errors are handled
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
