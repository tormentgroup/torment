use crate::types::auth::AuthState;
use matrix_sdk::ClientBuildError;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize, Clone)]
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
