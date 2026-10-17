//! The one door to the network. A trait, so tests use a fake and no test touches Google.

use std::time::Duration;

use crate::DriveError;

#[derive(Clone, Debug, Default)]
pub struct Request {
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }

    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

pub trait Http: Send + Sync {
    /// A transport failure (no network, TLS, timeout) is `Err`; an HTTP error status is a
    /// normal [`Response`].
    fn send(&self, req: &Request) -> Result<Response, DriveError>;
}

/// Responses bigger than this are refused (a .pptx with media can be large, but not this large).
const MAX_BODY: u64 = 512 * 1024 * 1024;

pub struct Ureq {
    agent: ureq::Agent,
}

impl Default for Ureq {
    fn default() -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(600)))
            .timeout_connect(Some(Duration::from_secs(15)))
            .user_agent("DeckCraft")
            .build()
            .into();
        Ureq { agent }
    }
}

impl Http for Ureq {
    fn send(&self, req: &Request) -> Result<Response, DriveError> {
        let offline = |e: ureq::Error| DriveError::Offline(e.to_string());
        let mut res = match req.method {
            "GET" | "DELETE" => {
                let mut b = if req.method == "GET" { self.agent.get(&req.url) } else { self.agent.delete(&req.url) };
                for (k, v) in &req.headers {
                    b = b.header(k, v);
                }
                b.call().map_err(offline)?
            }
            "POST" | "PUT" | "PATCH" => {
                let mut b = match req.method {
                    "POST" => self.agent.post(&req.url),
                    "PUT" => self.agent.put(&req.url),
                    _ => self.agent.patch(&req.url),
                };
                for (k, v) in &req.headers {
                    b = b.header(k, v);
                }
                b.send(&req.body[..]).map_err(offline)?
            }
            other => return Err(DriveError::Other(format!("unsupported method {other}"))),
        };
        let status = res.status().as_u16();
        let headers = res.headers().iter().filter_map(|(k, v)| v.to_str().ok().map(|v| (k.as_str().to_string(), v.to_string()))).collect();
        let body = res.body_mut().with_config().limit(MAX_BODY).read_to_vec().map_err(offline)?;
        Ok(Response { status, headers, body })
    }
}

#[cfg(test)]
pub mod fake {
    //! A scripted server: replies are queued, requests are recorded.
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    pub struct Fake {
        pub replies: Mutex<VecDeque<Result<Response, DriveError>>>,
        pub seen: Mutex<Vec<Request>>,
    }

    impl Fake {
        pub fn with(replies: Vec<Result<Response, DriveError>>) -> Self {
            Fake { replies: Mutex::new(replies.into()), seen: Mutex::default() }
        }

        pub fn json(status: u16, body: &str) -> Result<Response, DriveError> {
            Ok(Response { status, headers: vec![], body: body.as_bytes().to_vec() })
        }

        pub fn requests(&self) -> Vec<Request> {
            self.seen.lock().map(|g| g.clone()).unwrap_or_default()
        }
    }

    impl Http for Fake {
        fn send(&self, req: &Request) -> Result<Response, DriveError> {
            if let Ok(mut s) = self.seen.lock() {
                s.push(req.clone());
            }
            self.replies.lock().ok().and_then(|mut q| q.pop_front()).unwrap_or_else(|| Err(DriveError::Offline("no scripted reply".into())))
        }
    }
}
