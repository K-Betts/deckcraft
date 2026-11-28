//! The one door to the network, as a trait so tests use a fake.

use std::time::Duration;

use crate::UpdateError;

pub trait Http: Send + Sync {
    /// GET `url`; an HTTP error status is returned as `(status, body)`, a transport failure is `Err`.
    /// At most `limit` bytes of body are read.
    fn get(&self, url: &str, limit: u64) -> Result<(u16, Vec<u8>), UpdateError>;
}

pub struct Ureq {
    agent: ureq::Agent,
}

impl Default for Ureq {
    fn default() -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(900)))
            .timeout_connect(Some(Duration::from_secs(15)))
            .user_agent("DeckCraft")
            .build()
            .into();
        Ureq { agent }
    }
}

impl Http for Ureq {
    fn get(&self, url: &str, limit: u64) -> Result<(u16, Vec<u8>), UpdateError> {
        let offline = |e: ureq::Error| UpdateError::Offline(e.to_string());
        let mut res = self.agent.get(url).header("Accept", "application/vnd.github+json").call().map_err(offline)?;
        let status = res.status().as_u16();
        let body = res.body_mut().with_config().limit(limit).read_to_vec().map_err(offline)?;
        Ok((status, body))
    }
}

#[cfg(test)]
pub mod fake {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;

    /// URL → reply. Anything not listed is "offline".
    #[derive(Default)]
    pub struct Fake {
        pub replies: HashMap<String, (u16, Vec<u8>)>,
        pub seen: Mutex<Vec<String>>,
    }

    impl Fake {
        pub fn with(mut self, url: &str, status: u16, body: impl Into<Vec<u8>>) -> Self {
            self.replies.insert(url.to_string(), (status, body.into()));
            self
        }

        pub fn urls(&self) -> Vec<String> {
            self.seen.lock().map(|g| g.clone()).unwrap_or_default()
        }
    }

    impl Http for Fake {
        fn get(&self, url: &str, limit: u64) -> Result<(u16, Vec<u8>), UpdateError> {
            if let Ok(mut s) = self.seen.lock() {
                s.push(url.to_string());
            }
            let (status, body) = self.replies.get(url).cloned().ok_or_else(|| UpdateError::Offline("no network".into()))?;
            if body.len() as u64 > limit {
                return Err(UpdateError::Offline("too large".into()));
            }
            Ok((status, body))
        }
    }
}
