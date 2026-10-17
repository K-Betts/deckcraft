//! The loopback redirect: a one-shot listener on 127.0.0.1 that catches Google's redirect.
//!
//! The same pattern as the control server (`apps/deckcraft/src/control_server.rs`): loopback
//! only, one connection, no framework. It answers the browser with a plain page and closes.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

use crate::DriveError;
use crate::oauth::dec;

pub struct Listener {
    listener: TcpListener,
    pub port: u16,
}

impl Listener {
    /// Bind a random free port on the loopback interface only.
    pub fn bind() -> Result<Self, DriveError> {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
            .map_err(|e| DriveError::Other(format!("cannot listen for the sign-in: {e}")))?;
        let port = listener.local_addr().map_err(|e| DriveError::Other(e.to_string()))?.port();
        listener.set_nonblocking(true).map_err(|e| DriveError::Other(e.to_string()))?;
        Ok(Listener { listener, port })
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Wait up to `timeout` for the redirect; return the `code` once `state` matches.
    pub fn wait_for_code(&self, expected_state: &str, timeout: Duration, cancelled: impl Fn() -> bool) -> Result<String, DriveError> {
        let start = Instant::now();
        loop {
            if cancelled() {
                return Err(DriveError::Cancelled("cancelled".into()));
            }
            if start.elapsed() > timeout {
                return Err(DriveError::Cancelled("the browser window was not finished in time".into()));
            }
            match self.listener.accept() {
                Ok((stream, _)) => match handle(stream, expected_state) {
                    Handled::Code(c) => return Ok(c),
                    Handled::Failed(e) => return Err(e),
                    Handled::Ignore => {}
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(100)),
                Err(e) => return Err(DriveError::Other(format!("sign-in listener: {e}"))),
            }
        }
    }
}

enum Handled {
    Code(String),
    Failed(DriveError),
    /// Not our redirect (a favicon request, a probe): keep waiting.
    Ignore,
}

const PAGE: &str = "<!doctype html><meta charset=utf-8><title>DeckCraft</title><body style=\"font-family:system-ui;margin:3em\"><h2>You can close this tab</h2><p>Return to DeckCraft.</p></body>";

fn reply(mut stream: TcpStream, status: &str) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{PAGE}",
        PAGE.len()
    );
    let _ = stream.flush();
}

fn handle(stream: TcpStream, expected_state: &str) -> Handled {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut line = String::new();
    let Ok(clone) = stream.try_clone() else { return Handled::Ignore };
    if BufReader::new(clone).take(8192).read_line(&mut line).is_err() {
        return Handled::Ignore;
    }
    match parse_request_line(&line, expected_state) {
        Some(h) => {
            reply(stream, "200 OK");
            h
        }
        None => {
            reply(stream, "404 Not Found");
            Handled::Ignore
        }
    }
}

use std::io::Read as _;

/// `GET /?code=…&state=… HTTP/1.1`. A wrong `state` is an error, not "keep waiting": someone
/// else's request must not be able to supply a code for our sign-in.
fn parse_request_line(line: &str, expected_state: &str) -> Option<Handled> {
    let target = line.strip_prefix("GET ")?.split_whitespace().next()?;
    let query = target.strip_prefix("/?")?;
    let (mut code, mut state, mut error) = (None, None, None);
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k {
                "code" => code = Some(dec(v)),
                "state" => state = Some(dec(v)),
                "error" => error = Some(dec(v)),
                _ => {}
            }
        }
    }
    if state.as_deref() != Some(expected_state) {
        return Some(Handled::Failed(DriveError::Cancelled("the sign-in reply did not match the request, so it was ignored".into())));
    }
    if let Some(e) = error {
        return Some(Handled::Failed(DriveError::Cancelled(if e == "access_denied" { "you chose not to allow access".into() } else { e })));
    }
    code.filter(|c| !c.is_empty()).map(Handled::Code)
}

/// Open the sign-in page in the teacher's own browser.
pub fn open_in_browser(url: &str) -> Result<(), DriveError> {
    if !url.starts_with("https://accounts.google.com/") {
        return Err(DriveError::Other("refusing to open an unexpected address".into()));
    }
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    std::process::Command::new(program).arg(url).spawn().map(|_| ()).map_err(|e| DriveError::Other(format!("cannot open the browser: {e}")))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn visit(port: u16, path: &str) {
        let mut s = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        write!(s, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut sink = String::new();
        let _ = s.read_to_string(&mut sink);
    }

    #[test]
    fn the_redirect_with_the_right_state_yields_the_code() {
        let l = Listener::bind().unwrap();
        let port = l.port;
        let t = std::thread::spawn(move || visit(port, "/favicon.ico"));
        let t2 = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            visit(port, "/?state=S%21&code=4%2Fabc");
        });
        let code = l.wait_for_code("S!", Duration::from_secs(5), || false).unwrap();
        assert_eq!(code, "4/abc");
        t.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn a_wrong_state_is_refused() {
        let l = Listener::bind().unwrap();
        let port = l.port;
        let t = std::thread::spawn(move || visit(port, "/?state=evil&code=stolen"));
        let err = l.wait_for_code("good", Duration::from_secs(5), || false).unwrap_err();
        assert!(matches!(err, DriveError::Cancelled(_)));
        t.join().unwrap();
    }

    #[test]
    fn denying_access_and_timing_out_are_cancellations() {
        let l = Listener::bind().unwrap();
        let port = l.port;
        let t = std::thread::spawn(move || visit(port, "/?state=s&error=access_denied"));
        let err = l.wait_for_code("s", Duration::from_secs(5), || false).unwrap_err();
        assert_eq!(err, DriveError::Cancelled("you chose not to allow access".into()));
        t.join().unwrap();
        let l2 = Listener::bind().unwrap();
        assert!(matches!(l2.wait_for_code("s", Duration::from_millis(250), || false), Err(DriveError::Cancelled(_))));
        assert!(matches!(l2.wait_for_code("s", Duration::from_secs(5), || true), Err(DriveError::Cancelled(_))));
    }

    #[test]
    fn it_listens_on_loopback_only_and_opens_only_google() {
        let l = Listener::bind().unwrap();
        assert_eq!(l.listener.local_addr().unwrap().ip(), Ipv4Addr::LOCALHOST);
        assert!(open_in_browser("https://evil.example/").is_err());
    }
}
