//! Google Drive for DeckCraft.
//!
//! What this crate does, and what it will not do:
//!
//! - **Scope: `drive.file` only.** The app sees only files it created or the teacher explicitly
//!   picked. It cannot read the rest of a Drive.
//! - **Sign-in is opt-in** and starts only from a visible button. The flow is the installed-app
//!   loopback redirect with PKCE: a one-shot listener on `127.0.0.1`, a random `state`, and a
//!   code verifier that never leaves the machine except in the token request.
//! - **The client ID is configuration**, never compiled in: see [`config`].
//! - **Tokens live in the system keychain**, not in `prefs.json`. Signing out deletes them
//!   (and asks Google to revoke the grant).
//! - Nothing is uploaded unless the teacher presses Save to Google Drive.
//! - No telemetry. The only hosts contacted are `accounts.google.com`, `oauth2.googleapis.com`
//!   and `www.googleapis.com`.
//!
//! All network traffic goes through the [`http::Http`] trait, so every flow is tested against a
//! fake server.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#![forbid(unsafe_code)]

pub mod api;
pub mod config;
pub mod http;
pub mod oauth;
pub mod service;
pub mod signin;
pub mod store;

pub use api::{Drive, DriveFile};
pub use config::{Config, ConfigError};
pub use service::{DriveEvent, DriveService, DriveStatus};

/// Everything that can go wrong, in words a teacher can act on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DriveError {
    /// No client ID has been configured, so sign-in cannot start.
    NotConfigured(String),
    /// Not signed in (or the grant was revoked or expired for good).
    SignedOut,
    /// No network, or Google could not be reached.
    Offline(String),
    /// Google refused: the message is Google's own.
    Refused(String),
    /// The file changed in Drive since it was opened; saving would overwrite someone's work.
    Conflict,
    /// The sign-in window was closed, timed out or was denied.
    Cancelled(String),
    /// Anything else (a keychain error, a malformed reply).
    Other(String),
}

impl std::fmt::Display for DriveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriveError::NotConfigured(m) => write!(f, "Google Drive is not set up: {m}"),
            DriveError::SignedOut => write!(f, "Not signed in to Google Drive."),
            DriveError::Offline(m) => write!(f, "Can't reach Google Drive (no network?): {m}"),
            DriveError::Refused(m) => write!(f, "Google Drive said no: {m}"),
            DriveError::Conflict => write!(f, "This file changed in Google Drive after you opened it. Save a copy instead."),
            DriveError::Cancelled(m) => write!(f, "Sign-in did not finish: {m}"),
            DriveError::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for DriveError {}

/// Seconds since the Unix epoch.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
