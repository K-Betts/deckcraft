//! OAuth 2.0 for an installed app: PKCE, the authorisation URL and the token requests.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::http::{Http, Request, Response};
use crate::{DriveError, now_secs};

/// The only scope DeckCraft asks for: files it created or the teacher picked.
pub const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
pub const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
pub const REVOKE_ENDPOINT: &str = "https://oauth2.googleapis.com/revoke";

/// What the keychain holds.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Seconds since the epoch at which `access_token` stops working.
    pub expires_at: u64,
}

impl std::fmt::Debug for Tokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tokens {{ expires_at: {}, .. }}", self.expires_at)
    }
}

impl Tokens {
    /// Treat a token as expired a minute early so a request never races its expiry.
    pub fn expired(&self, now: u64) -> bool {
        now + 60 >= self.expires_at
    }
}

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

fn random_bytes<const N: usize>() -> Result<[u8; N], DriveError> {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).map_err(|e| DriveError::Other(format!("no secure random numbers: {e}")))?;
    Ok(b)
}

impl Pkce {
    pub fn new() -> Result<Self, DriveError> {
        let verifier = URL_SAFE_NO_PAD.encode(random_bytes::<48>()?);
        Ok(Pkce { challenge: challenge_of(&verifier), verifier })
    }
}

/// `BASE64URL(SHA256(verifier))`, RFC 7636 §4.2.
pub fn challenge_of(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// An unguessable value that ties the redirect to this sign-in.
pub fn new_state() -> Result<String, DriveError> {
    Ok(URL_SAFE_NO_PAD.encode(random_bytes::<24>()?))
}

/// Percent-encode a query value.
pub fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn dec(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if b.get(i + 1..i + 3).is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit)) => {
                let hex = std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("00");
                out.push(u8::from_str_radix(hex, 16).unwrap_or(b'?'));
                i += 2;
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn auth_url(cfg: &Config, redirect_uri: &str, state: &str, challenge: &str) -> String {
    format!(
        "{AUTH_ENDPOINT}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
        enc(&cfg.client_id),
        enc(redirect_uri),
        enc(SCOPE),
        enc(state),
        enc(challenge),
    )
}

fn form(pairs: &[(&str, &str)]) -> String {
    pairs.iter().map(|(k, v)| format!("{}={}", enc(k), enc(v))).collect::<Vec<_>>().join("&")
}

fn post_form(url: &str, pairs: &[(&str, &str)]) -> Request {
    Request {
        method: "POST",
        url: url.into(),
        headers: vec![("Content-Type".into(), "application/x-www-form-urlencoded".into())],
        body: form(pairs).into_bytes(),
    }
}

#[derive(Deserialize)]
struct TokenReply {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

fn parse_tokens(res: &Response, keep_refresh: Option<&str>) -> Result<Tokens, DriveError> {
    let reply: TokenReply =
        serde_json::from_slice(&res.body).map_err(|_| DriveError::Other(format!("unreadable reply from Google (status {})", res.status)))?;
    if let Some(e) = reply.error {
        // `invalid_grant` is what a revoked, expired or reused refresh token returns: signed out for good.
        return Err(if e == "invalid_grant" { DriveError::SignedOut } else { DriveError::Refused(reply.error_description.unwrap_or(e)) });
    }
    let access = reply.access_token.ok_or_else(|| DriveError::Other("Google sent no access token".into()))?;
    let refresh =
        reply.refresh_token.or_else(|| keep_refresh.map(String::from)).ok_or_else(|| DriveError::Other("Google sent no refresh token".into()))?;
    Ok(Tokens { access_token: access, refresh_token: refresh, expires_at: now_secs() + reply.expires_in.unwrap_or(3000) })
}

/// Trade the one-time code for tokens.
pub fn exchange_code(http: &dyn Http, cfg: &Config, code: &str, verifier: &str, redirect_uri: &str) -> Result<Tokens, DriveError> {
    let mut pairs = vec![
        ("client_id", cfg.client_id.as_str()),
        ("code", code),
        ("code_verifier", verifier),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
    ];
    if let Some(s) = cfg.client_secret.as_deref() {
        pairs.push(("client_secret", s));
    }
    parse_tokens(&http.send(&post_form(TOKEN_ENDPOINT, &pairs))?, None)
}

pub fn refresh(http: &dyn Http, cfg: &Config, refresh_token: &str) -> Result<Tokens, DriveError> {
    let mut pairs = vec![("client_id", cfg.client_id.as_str()), ("refresh_token", refresh_token), ("grant_type", "refresh_token")];
    if let Some(s) = cfg.client_secret.as_deref() {
        pairs.push(("client_secret", s));
    }
    parse_tokens(&http.send(&post_form(TOKEN_ENDPOINT, &pairs))?, Some(refresh_token))
}

/// Ask Google to forget the grant. Best effort: signing out never depends on it.
pub fn revoke(http: &dyn Http, token: &str) {
    let _ = http.send(&post_form(REVOKE_ENDPOINT, &[("token", token)]));
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::http::fake::Fake;

    fn cfg() -> Config {
        Config { client_id: "cid".into(), client_secret: Some("sec".into()) }
    }

    #[test]
    fn the_challenge_matches_the_rfc_7636_example() {
        // RFC 7636 appendix B.
        assert_eq!(challenge_of("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn verifiers_and_states_are_long_url_safe_and_fresh() {
        let a = Pkce::new().unwrap();
        let b = Pkce::new().unwrap();
        assert_ne!(a.verifier, b.verifier);
        assert!((43..=128).contains(&a.verifier.len()), "RFC 7636 length");
        assert!(a.verifier.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_ne!(new_state().unwrap(), new_state().unwrap());
    }

    #[test]
    fn the_auth_url_asks_for_drive_file_only_with_pkce_and_the_loopback_redirect() {
        let url = auth_url(&cfg(), "http://127.0.0.1:5555", "st", "ch");
        assert!(url.starts_with("https://accounts.google.com/"));
        assert!(url.contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fdrive.file&"), "{url}");
        assert!(!url.contains("drive.readonly") && !url.contains("auth%2Fdrive&"), "no wider scope");
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A5555"));
        assert!(url.contains("code_challenge=ch&code_challenge_method=S256"));
        assert!(url.contains("state=st"));
        assert!(!url.contains("sec"), "the secret never goes in a URL");
    }

    #[test]
    fn percent_coding_round_trips() {
        for s in ["a b&c=d/é", "plain", "100%"] {
            assert_eq!(dec(&enc(s)), s);
        }
        assert_eq!(dec("a+b%2Fc%zz"), "a b/c%zz");
    }

    #[test]
    fn exchanging_a_code_sends_the_verifier_and_stores_both_tokens() {
        let http = Fake::with(vec![Fake::json(200, r#"{"access_token":"A","refresh_token":"R","expires_in":3600}"#)]);
        let t = exchange_code(&http, &cfg(), "the-code", "the-verifier", "http://127.0.0.1:1").unwrap();
        assert_eq!((t.access_token.as_str(), t.refresh_token.as_str()), ("A", "R"));
        assert!(t.expires_at > now_secs() + 3000);
        let body = String::from_utf8(http.requests()[0].body.clone()).unwrap();
        assert!(body.contains("code_verifier=the-verifier") && body.contains("grant_type=authorization_code") && body.contains("client_secret=sec"));
    }

    #[test]
    fn a_refresh_keeps_the_old_refresh_token_when_google_sends_none() {
        let http = Fake::with(vec![Fake::json(200, r#"{"access_token":"B","expires_in":100}"#)]);
        let t = refresh(&http, &cfg(), "R-old").unwrap();
        assert_eq!((t.access_token.as_str(), t.refresh_token.as_str()), ("B", "R-old"));
        assert!(t.expired(now_secs() + 100), "a token with 100 s left counts as expired a minute early");
    }

    #[test]
    fn a_revoked_grant_means_signed_out_and_other_errors_carry_googles_words() {
        let http = Fake::with(vec![
            Fake::json(400, r#"{"error":"invalid_grant"}"#),
            Fake::json(400, r#"{"error":"access_denied","error_description":"Nope"}"#),
        ]);
        assert_eq!(refresh(&http, &cfg(), "R").unwrap_err(), DriveError::SignedOut);
        assert_eq!(refresh(&http, &cfg(), "R").unwrap_err(), DriveError::Refused("Nope".into()));
    }

    #[test]
    fn offline_is_reported_as_offline() {
        let http = Fake::with(vec![Err(DriveError::Offline("dns".into()))]);
        assert!(matches!(refresh(&http, &cfg(), "R"), Err(DriveError::Offline(_))));
    }

    #[test]
    fn tokens_never_print() {
        let t = Tokens { access_token: "SECRETA".into(), refresh_token: "SECRETR".into(), expires_at: 1 };
        assert!(!format!("{t:?}").contains("SECRET"));
    }
}
