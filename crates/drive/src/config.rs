//! Where the OAuth client ID comes from. Never compiled in, never invented.
//!
//! Create it once in the Google Cloud Console (APIs & Services → Credentials → OAuth client ID →
//! **Desktop app**), then give it to DeckCraft by either:
//!
//! - environment variables `DECKCRAFT_GOOGLE_CLIENT_ID` (and `DECKCRAFT_GOOGLE_CLIENT_SECRET`), or
//! - a file `~/.config/deckcraft/drive.toml`:
//!
//!   ```toml
//!   client_id = "1234-abc.apps.googleusercontent.com"
//!   client_secret = "GOCSPX-…"   # optional, see below
//!   ```
//!
//! Google issues a "client secret" for Desktop-app clients and its token endpoint expects it, but
//! Google itself documents that this secret cannot be kept confidential in an installed app. The
//! security comes from PKCE and the loopback redirect, not from the secret. It is read from the
//! same place and never logged.

use std::path::{Path, PathBuf};

#[derive(Clone, PartialEq, Eq)]
pub struct Config {
    pub client_id: String,
    pub client_secret: Option<String>,
}

// Never print the secret.
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config").field("client_id", &self.client_id).field("client_secret", &self.client_secret.as_ref().map(|_| "…")).finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    Missing,
    Malformed(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Missing => write!(
                f,
                "no Google client ID. Create a Desktop-app OAuth client in the Google Cloud Console, then set DECKCRAFT_GOOGLE_CLIENT_ID or write client_id to ~/.config/deckcraft/drive.toml"
            ),
            ConfigError::Malformed(m) => write!(f, "drive.toml: {m}"),
        }
    }
}

/// Where the config file is looked for, best first.
pub fn config_paths(home: Option<&Path>) -> Vec<PathBuf> {
    home.map(|h| vec![h.join(".config").join("deckcraft").join("drive.toml")]).unwrap_or_default()
}

/// Read the client from the environment, else the first config file that exists.
pub fn load(env: impl Fn(&str) -> Option<String>, home: Option<&Path>) -> Result<Config, ConfigError> {
    if let Some(id) = env("DECKCRAFT_GOOGLE_CLIENT_ID").filter(|s| !s.trim().is_empty()) {
        return validate(Config {
            client_id: id.trim().to_string(),
            client_secret: env("DECKCRAFT_GOOGLE_CLIENT_SECRET").filter(|s| !s.trim().is_empty()),
        });
    }
    for p in config_paths(home) {
        if let Ok(text) = std::fs::read_to_string(&p) {
            return parse(&text);
        }
    }
    Err(ConfigError::Missing)
}

pub fn from_process() -> Result<Config, ConfigError> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    load(|k| std::env::var(k).ok(), home.as_deref())
}

/// A deliberately tiny reader: `key = "value"` lines, `#` comments. Not a TOML parser.
pub fn parse(text: &str) -> Result<Config, ConfigError> {
    let (mut id, mut secret) = (None, None);
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (k, v) = line.split_once('=').ok_or_else(|| ConfigError::Malformed(format!("cannot read “{line}”")))?;
        let v = v.trim().trim_matches('"').trim().to_string();
        match k.trim() {
            "client_id" => id = Some(v),
            "client_secret" => secret = Some(v).filter(|s| !s.is_empty()),
            _ => {}
        }
    }
    validate(Config { client_id: id.ok_or(ConfigError::Missing)?, client_secret: secret })
}

fn validate(c: Config) -> Result<Config, ConfigError> {
    if c.client_id.is_empty() || c.client_id.len() > 256 || c.client_id.chars().any(|ch| ch.is_whitespace() || ch.is_control()) {
        return Err(ConfigError::Malformed("client_id is empty or has spaces".into()));
    }
    Ok(c)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn nothing_configured_is_missing_not_invented() {
        assert_eq!(load(|_| None, None), Err(ConfigError::Missing));
        assert_eq!(load(|_| None, Some(Path::new("/nonexistent-home"))), Err(ConfigError::Missing));
    }

    #[test]
    fn the_environment_wins_and_the_secret_is_optional() {
        let env = |k: &str| match k {
            "DECKCRAFT_GOOGLE_CLIENT_ID" => Some(" abc.apps.googleusercontent.com ".to_string()),
            _ => None,
        };
        let c = load(env, None).unwrap();
        assert_eq!(c.client_id, "abc.apps.googleusercontent.com");
        assert_eq!(c.client_secret, None);
    }

    #[test]
    fn the_file_reads_quoted_values_and_comments() {
        let c = parse("# my client\nclient_id = \"id-1\"\nclient_secret = \"s3\" # not secret\n").unwrap();
        assert_eq!((c.client_id.as_str(), c.client_secret.as_deref()), ("id-1", Some("s3")));
        assert_eq!(parse("client_secret = \"x\"").unwrap_err(), ConfigError::Missing);
        assert!(matches!(parse("client_id id"), Err(ConfigError::Malformed(_))));
        assert!(matches!(parse("client_id = \"a b\""), Err(ConfigError::Malformed(_))));
    }

    #[test]
    fn the_secret_never_appears_in_debug_output() {
        let c = Config { client_id: "id".into(), client_secret: Some("hunter2".into()) };
        assert!(!format!("{c:?}").contains("hunter2"));
    }
}
