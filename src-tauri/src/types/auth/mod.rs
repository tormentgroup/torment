pub mod error;
use serde::Serialize;

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
