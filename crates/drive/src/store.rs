//! Where the tokens are kept: the system keychain, never `prefs.json`.

use std::sync::Mutex;

use crate::DriveError;
use crate::oauth::Tokens;

pub trait TokenStore: Send + Sync {
    fn load(&self) -> Result<Option<Tokens>, DriveError>;
    fn save(&self, tokens: &Tokens) -> Result<(), DriveError>;
    /// Signing out: the tokens must be gone afterwards.
    fn delete(&self) -> Result<(), DriveError>;
}

const SERVICE: &str = "DeckCraft Google Drive";
const ACCOUNT: &str = "tokens";

/// The macOS Keychain (Credential Manager / Secret Service elsewhere).
pub struct Keychain;

fn entry() -> Result<keyring::Entry, DriveError> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| DriveError::Other(format!("keychain: {e}")))
}

impl TokenStore for Keychain {
    fn load(&self) -> Result<Option<Tokens>, DriveError> {
        match entry()?.get_password() {
            Ok(text) => Ok(serde_json::from_str(&text).ok()),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(DriveError::Other(format!("keychain: {e}"))),
        }
    }

    fn save(&self, tokens: &Tokens) -> Result<(), DriveError> {
        let text = serde_json::to_string(tokens).map_err(|e| DriveError::Other(e.to_string()))?;
        entry()?.set_password(&text).map_err(|e| DriveError::Other(format!("keychain: {e}")))
    }

    fn delete(&self) -> Result<(), DriveError> {
        match entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(DriveError::Other(format!("keychain: {e}"))),
        }
    }
}

/// For tests.
#[derive(Default)]
pub struct Memory(pub Mutex<Option<Tokens>>);

impl TokenStore for Memory {
    fn load(&self) -> Result<Option<Tokens>, DriveError> {
        Ok(self.0.lock().map_err(|_| DriveError::Other("poisoned".into()))?.clone())
    }

    fn save(&self, tokens: &Tokens) -> Result<(), DriveError> {
        *self.0.lock().map_err(|_| DriveError::Other("poisoned".into()))? = Some(tokens.clone());
        Ok(())
    }

    fn delete(&self) -> Result<(), DriveError> {
        *self.0.lock().map_err(|_| DriveError::Other("poisoned".into()))? = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn tokens() -> Tokens {
        Tokens { access_token: "a".into(), refresh_token: "r".into(), expires_at: 42 }
    }

    #[test]
    fn the_memory_store_round_trips_and_deletes() {
        let m = Memory::default();
        assert!(m.load().unwrap().is_none());
        m.save(&tokens()).unwrap();
        assert_eq!(m.load().unwrap().unwrap().expires_at, 42);
        m.delete().unwrap();
        assert!(m.load().unwrap().is_none());
        m.delete().unwrap();
    }

    /// Touches the REAL system keychain (macOS asks permission), so it is not run by default:
    /// `cargo test -p deckcraft-drive -- --ignored real_keychain`. It proves the tokens survive
    /// in the OS store rather than a mock (keyring v3 falls back to a mock with no platform
    /// feature). It saves under the real entry name, so it refuses to run while signed in.
    #[test]
    #[ignore = "uses the real system keychain"]
    fn real_keychain_round_trip() {
        let k = Keychain;
        assert!(k.load().unwrap().is_none(), "sign out of Drive first: this test would overwrite the real tokens");
        k.save(&tokens()).unwrap();
        assert_eq!(Keychain.load().unwrap().unwrap().refresh_token, "r");
        k.delete().unwrap();
        assert!(Keychain.load().unwrap().is_none());
    }
}
