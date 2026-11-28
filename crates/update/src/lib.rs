//! In-app updates for DeckCraft.
//!
//! The teacher presses a button; nothing here runs on its own. What it does and does not do:
//!
//! - **Look:** one HTTPS request to `api.github.com` for the latest release of this fork. It sends
//!   only the usual HTTP headers (no id, no version, no account).
//! - **Download:** the release's Mac app (a zip) from GitHub, and its SHA-256 file. The zip is
//!   refused unless its checksum matches. (The checksum guards against a damaged download, not a
//!   hostile release: trust rests on HTTPS and on who can publish releases to the repository.)
//! - **Install:** unzip, check the code signature is intact, move the old app aside, move the new
//!   one in, and relaunch. If any step fails the old app is put back.
//! - macOS only for now. Other systems get a clear "not yet" message, never a half-update.
//!
//! All network traffic goes through [`net::Http`], so every flow is tested against a fake.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#![forbid(unsafe_code)]

pub mod install;
pub mod net;
pub mod release;
pub mod service;

/// Everything that can go wrong, in words a teacher can act on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateError {
    /// No network, or GitHub could not be reached.
    Offline(String),
    /// GitHub answered, but not with a usable release.
    NoRelease(String),
    /// This build cannot be updated this way (a development build, or not macOS).
    Unsupported(String),
    /// The download did not match its checksum, or was too large.
    BadDownload(String),
    /// Unzip, signature check or swap failed. The old app was kept.
    Install(String),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::Offline(m) => write!(f, "Can't reach GitHub (no network?): {m}"),
            UpdateError::NoRelease(m) => write!(f, "No update found: {m}"),
            UpdateError::Unsupported(m) => write!(f, "{m}"),
            UpdateError::BadDownload(m) => write!(f, "The download could not be trusted, so it was not installed: {m}"),
            UpdateError::Install(m) => write!(f, "The update could not be installed (your current version is untouched): {m}"),
        }
    }
}

impl std::error::Error for UpdateError {}
