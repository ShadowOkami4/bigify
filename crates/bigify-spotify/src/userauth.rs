//! Library access through the user's own Spotify developer app.
//!
//! Spotify rate-limits third-party use of its own client's tokens on the
//! Web API (HTTP 429 with retry times of hours), so home, search and
//! library need an app registered by the user. Setup is a one-time browser
//! flow served by Bigify itself:
//!
//! 1. `http://127.0.0.1:8899/` explains how to create the app and asks for
//!    its Client ID (typing happens in a browser, not on the TV UI).
//! 2. Bigify redirects to Spotify's consent page (OAuth 2.0 + PKCE; no
//!    client secret is involved).
//! 3. Spotify redirects back to `/callback`; the code is exchanged for an
//!    access + refresh token stored in `$XDG_STATE_HOME/bigify` (mode 0600).

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bigify_core::error::{ErrorKind, ServiceError, ServiceResult};
use bytes::Bytes;
use librespot::core::session::Session;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

use crate::errors::from_librespot;
use crate::webapi::encode;

pub const SETUP_PORT: u16 = 8899;
const AUTHORIZE_URL: &str = "https://accounts.spotify.com/authorize";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
/// The setup page closes itself after this long without completion.
const SETUP_TIMEOUT: Duration = Duration::from_secs(20 * 60);

/// Scopes for the user's app (Web API only; playback uses librespot).
const SCOPES: &str = "user-read-private user-library-read user-library-modify \
user-read-recently-played user-top-read user-follow-read user-follow-modify \
playlist-read-private playlist-read-collaborative playlist-modify-public \
playlist-modify-private user-read-playback-state user-modify-playback-state";

pub fn setup_url() -> String {
    format!("http://127.0.0.1:{SETUP_PORT}/")
}

pub fn redirect_uri() -> String {
    format!("http://127.0.0.1:{SETUP_PORT}/callback")
}

/// Spotify Client IDs are 32 hexadecimal characters.
pub fn valid_client_id(id: &str) -> bool {
    id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserToken {
    pub client_id: String,
    pub access_token: String,
    pub refresh_token: String,
    /// Unix seconds.
    pub expires_at: u64,
}

impl std::fmt::Debug for UserToken {
    // Never print tokens.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserToken")
            .field("client_id", &self.client_id)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

impl UserToken {
    pub fn is_expired(&self) -> bool {
        now_secs() + 60 >= self.expires_at
    }

    pub fn load(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<Self>(&text)
            .ok()
            .filter(|t| valid_client_id(&t.client_id) && !t.refresh_token.is_empty())
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(self).unwrap_or_default())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::rename(&tmp, path)
    }
}

/// Token shared by the engine and Web API requests.
pub struct UserAuth {
    pub path: PathBuf,
    pub token: tokio::sync::Mutex<Option<UserToken>>,
}

impl UserAuth {
    pub fn load(path: PathBuf) -> Arc<Self> {
        let token = UserToken::load(&path);
        if token.is_some() {
            log::info!("library access: using your Spotify app");
        }
        Arc::new(Self {
            path,
            token: tokio::sync::Mutex::new(token),
        })
    }

    pub async fn is_configured(&self) -> bool {
        self.token.lock().await.is_some()
    }

    pub async fn set(&self, token: UserToken) -> std::io::Result<()> {
        token.save(&self.path)?;
        *self.token.lock().await = Some(token);
        Ok(())
    }

    pub async fn clear(&self) {
        *self.token.lock().await = None;
        match std::fs::remove_file(&self.path) {
            Ok(()) => log::info!("library access removed"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => log::warn!("could not remove library token: {e}"),
        }
    }

    /// A valid access token, refreshing it when needed. `None` when no app
    /// is configured.
    pub async fn access_token(&self, session: &Session) -> Option<ServiceResult<String>> {
        let mut guard = self.token.lock().await;
        let current = guard.as_ref()?.clone();
        if !current.is_expired() {
            return Some(Ok(current.access_token));
        }
        match refresh(session, &current).await {
            Ok(fresh) => {
                if let Err(e) = fresh.save(&self.path) {
                    log::warn!("could not store refreshed library token: {e}");
                }
                let access = fresh.access_token.clone();
                *guard = Some(fresh);
                Some(Ok(access))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

// ---------------------------------------------------------------------------
// OAuth 2.0 authorization code flow with PKCE

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
    pub state: String,
}

fn random_urlsafe(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    if getrandom::fill(&mut buf).is_err() {
        // Extremely unlikely; fall back to time-based entropy so setup still
        // works, but log it.
        log::warn!("system randomness unavailable");
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        for (i, b) in buf.iter_mut().enumerate() {
            *b = (seed >> ((i % 16) * 8)) as u8 ^ (i as u8).wrapping_mul(31);
        }
    }
    data_encoding::BASE64URL_NOPAD.encode(&buf)
}

pub fn pkce() -> Pkce {
    let verifier = random_urlsafe(64);
    let challenge = data_encoding::BASE64URL_NOPAD.encode(&Sha256::digest(verifier.as_bytes()));
    Pkce {
        verifier,
        challenge,
        state: random_urlsafe(16),
    }
}

pub fn authorize_url(client_id: &str, pkce: &Pkce) -> String {
    format!(
        "{AUTHORIZE_URL}?response_type=code&client_id={}&redirect_uri={}&scope={}\
         &code_challenge_method=S256&code_challenge={}&state={}",
        encode(client_id),
        encode(&redirect_uri()),
        encode(SCOPES),
        encode(&pkce.challenge),
        encode(&pkce.state),
    )
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

async fn token_request(session: &Session, form: String) -> ServiceResult<TokenResponse> {
    let req = http::Request::post(TOKEN_URL)
        .header(
            http::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .header(http::header::ACCEPT, "application/json")
        .body(Bytes::from(form))
        .map_err(|e| ServiceError::internal(format!("Invalid token request ({e})")))?;
    let body = session.http_client().request_body(req).await.map_err(|e| {
        log::warn!("Spotify token request failed: {e}");
        match from_librespot(e) {
            err if err.kind == ErrorKind::Network => err,
            _ => ServiceError::new(
                ErrorKind::Unauthorized,
                "Spotify didn't accept the sign-in for your app. Run the setup again.",
            ),
        }
    })?;
    serde_json::from_slice(&body)
        .map_err(|_| ServiceError::internal("Unexpected token response from Spotify"))
}

pub async fn exchange_code(
    session: &Session,
    client_id: &str,
    code: &str,
    verifier: &str,
) -> ServiceResult<UserToken> {
    let form = format!(
        "grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&code_verifier={}",
        encode(code),
        encode(&redirect_uri()),
        encode(client_id),
        encode(verifier),
    );
    let r = token_request(session, form).await?;
    let refresh_token = r
        .refresh_token
        .ok_or_else(|| ServiceError::internal("Spotify returned no refresh token"))?;
    Ok(UserToken {
        client_id: client_id.to_owned(),
        access_token: r.access_token,
        refresh_token,
        expires_at: now_secs() + r.expires_in.unwrap_or(3600),
    })
}

pub async fn refresh(session: &Session, token: &UserToken) -> ServiceResult<UserToken> {
    let form = format!(
        "grant_type=refresh_token&refresh_token={}&client_id={}",
        encode(&token.refresh_token),
        encode(&token.client_id),
    );
    let r = token_request(session, form).await?;
    Ok(UserToken {
        client_id: token.client_id.clone(),
        access_token: r.access_token,
        // Spotify may rotate refresh tokens.
        refresh_token: r
            .refresh_token
            .unwrap_or_else(|| token.refresh_token.clone()),
        expires_at: now_secs() + r.expires_in.unwrap_or(3600),
    })
}

// ---------------------------------------------------------------------------
// Local setup page

#[derive(Debug)]
pub enum SetupEvent {
    /// The user approved; exchange `code` for tokens.
    Code {
        client_id: String,
        code: String,
        verifier: String,
    },
    Failed(String),
}

pub struct SetupServer {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl SetupServer {
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for SetupServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub fn start_setup_server(
    events: mpsc::UnboundedSender<SetupEvent>,
) -> std::io::Result<SetupServer> {
    // Loopback only: the page is reachable from this device alone.
    let listener = TcpListener::bind(("127.0.0.1", SETUP_PORT))?;
    listener.set_nonblocking(true)?;
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let thread = std::thread::Builder::new()
        .name("bigify-setup".into())
        .spawn(move || serve(listener, stop_flag, events))?;
    Ok(SetupServer {
        stop,
        thread: Some(thread),
    })
}

struct Pending {
    client_id: String,
    pkce: Pkce,
}

fn serve(listener: TcpListener, stop: Arc<AtomicBool>, events: mpsc::UnboundedSender<SetupEvent>) {
    let started = Instant::now();
    let mut pending: Option<Pending> = None;
    while !stop.load(Ordering::Relaxed) {
        if started.elapsed() > SETUP_TIMEOUT {
            let _ = events.send(SetupEvent::Failed("Setup timed out".into()));
            break;
        }
        match listener.accept() {
            Ok((stream, _)) => {
                if handle(stream, &mut pending, &events) {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                log::warn!("setup page: {e}");
                std::thread::sleep(Duration::from_millis(250));
            }
        }
    }
    log::debug!("setup page closed");
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then(|| percent_decode(v))
    })
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn respond(mut stream: TcpStream, status: &str, extra_headers: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n{extra_headers}\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

/// Handle one request. Returns true when setup is finished.
fn handle(
    stream: TcpStream,
    pending: &mut Option<Pending>,
    events: &mpsc::UnboundedSender<SetupEvent>,
) -> bool {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut line = String::new();
    if BufReader::new(&stream).read_line(&mut line).is_err() {
        return false;
    }
    let target = line.split_whitespace().nth(1).unwrap_or("/");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));

    match path {
        "/" => respond(stream, "200 OK", "", &page_form(None)),
        "/start" => {
            let client_id = query_param(query, "client_id")
                .unwrap_or_default()
                .trim()
                .to_owned();
            if !valid_client_id(&client_id) {
                respond(
                    stream,
                    "200 OK",
                    "",
                    &page_form(Some(
                        "That doesn't look like a Client ID. It has 32 letters and digits.",
                    )),
                );
                return false;
            }
            let p = pkce();
            let url = authorize_url(&client_id, &p);
            *pending = Some(Pending { client_id, pkce: p });
            respond(stream, "302 Found", &format!("Location: {url}\r\n"), "");
        }
        "/callback" => {
            let state = query_param(query, "state").unwrap_or_default();
            let Some(p) = pending.as_ref().filter(|p| p.pkce.state == state) else {
                respond(
                    stream,
                    "400 Bad Request",
                    "",
                    &page_message(
                        "Something didn't match",
                        "Start the setup again from Bigify's settings.",
                    ),
                );
                return false;
            };
            if let Some(error) = query_param(query, "error") {
                let _ = events.send(SetupEvent::Failed(format!("Spotify returned: {error}")));
                respond(stream, "200 OK", "", &page_message(
                    "Not connected",
                    "You declined access or Spotify reported an error. You can start the setup again from Bigify's settings.",
                ));
                return true;
            }
            let Some(code) = query_param(query, "code") else {
                respond(
                    stream,
                    "400 Bad Request",
                    "",
                    &page_message("Missing code", "Start the setup again."),
                );
                return false;
            };
            let _ = events.send(SetupEvent::Code {
                client_id: p.client_id.clone(),
                code,
                verifier: p.pkce.verifier.clone(),
            });
            respond(stream, "200 OK", "", &page_message(
                "Done",
                "Bigify is finishing the connection. You can close this tab and return to Bigify.",
            ));
            return true;
        }
        _ => respond(stream, "404 Not Found", "", &page_message("Not found", "")),
    }
    false
}

const PAGE_STYLE: &str = "body{font-family:system-ui,sans-serif;background:#141218;color:#e6e0e9;\
max-width:760px;margin:40px auto;padding:0 20px;line-height:1.55;font-size:18px}\
h1{color:#cfbcff}a{color:#cfbcff}code{background:#2b2930;padding:2px 8px;border-radius:6px;\
user-select:all}ol li{margin:10px 0}input{font-size:20px;padding:12px 16px;width:100%;\
box-sizing:border-box;border-radius:14px;border:2px solid #938f99;background:#1d1b20;color:#e6e0e9}\
button{margin-top:16px;font-size:20px;padding:14px 28px;border:0;border-radius:28px;\
background:#cfbcff;color:#381e72;font-weight:600;cursor:pointer}.err{color:#ffb4ab}\
.note{color:#cac4d0;font-size:16px}";

fn page_form(error: Option<&str>) -> String {
    let redirect = redirect_uri();
    let error = error
        .map(|e| format!("<p class=err>{}</p>", html_escape(e)))
        .unwrap_or_default();
    format!(
        "<!doctype html><meta charset=utf-8><meta name=viewport content='width=device-width'>\
<title>Bigify — library access</title><style>{PAGE_STYLE}</style>\
<h1>Connect Bigify to your library</h1>\
<p>Spotify only lets apps registered by you read your playlists, library and search. \
This is a one-time setup and takes about three minutes.</p>\
<ol>\
<li>Open the <a href='https://developer.spotify.com/dashboard' target=_blank rel=noopener>Spotify Developer Dashboard</a> \
and log in with your Spotify account.</li>\
<li>Choose <b>Create app</b>. Name: <code>Bigify</code>, description: anything.</li>\
<li>Redirect URI: <code>{redirect}</code> — then click <b>Add</b>.</li>\
<li>Under <b>Which API/SDKs are you planning to use?</b> tick <b>Web API</b>, accept the terms and <b>Save</b>.</li>\
<li>Copy the <b>Client ID</b> shown for the new app and paste it here:</li>\
</ol>\
<form action=/start method=get>{error}\
<input name=client_id placeholder='Client ID (32 characters)' autocomplete=off autofocus spellcheck=false>\
<button type=submit>Continue to Spotify</button></form>\
<p class=note>No password or secret is stored. Bigify keeps a revocable access token on this device; \
remove it any time in Bigify's settings or in your Spotify account under Manage apps.</p>"
    )
}

fn page_message(title: &str, text: &str) -> String {
    format!(
        "<!doctype html><meta charset=utf-8><title>Bigify</title><style>{PAGE_STYLE}</style>\
<h1>{}</h1><p>{}</p>",
        html_escape(title),
        html_escape(text)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_id_validation() {
        assert!(valid_client_id("0123456789abcdef0123456789ABCDEF"));
        assert!(!valid_client_id("0123456789abcdef"));
        assert!(!valid_client_id("0123456789abcdef0123456789abcdeg"));
    }

    #[test]
    fn pkce_is_rfc7636_shaped() {
        let p = pkce();
        assert!((43..=128).contains(&p.verifier.len()));
        assert!(p
            .verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        let expected =
            data_encoding::BASE64URL_NOPAD.encode(&Sha256::digest(p.verifier.as_bytes()));
        assert_eq!(p.challenge, expected);
        assert_ne!(pkce().verifier, p.verifier);
    }

    #[test]
    fn query_parsing() {
        let q = "code=AQB%2Fx%3D&state=abc&error=access_denied";
        assert_eq!(query_param(q, "code").as_deref(), Some("AQB/x="));
        assert_eq!(query_param(q, "state").as_deref(), Some("abc"));
        assert_eq!(query_param(q, "missing"), None);
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("a+b"), "a b");
    }

    #[test]
    fn authorize_url_contains_pkce_and_redirect() {
        let p = pkce();
        let url = authorize_url("0123456789abcdef0123456789abcdef", &p);
        assert!(url.starts_with(AUTHORIZE_URL));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains(&encode(&redirect_uri())));
        assert!(url.contains(&format!("state={}", p.state)));
    }

    fn http_get(path: &str) -> String {
        use std::io::Read;
        let mut s = TcpStream::connect(("127.0.0.1", SETUP_PORT)).unwrap();
        write!(s, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        out
    }

    #[test]
    fn setup_page_flow() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let server = start_setup_server(tx).unwrap();

        let page = http_get("/");
        assert!(page.starts_with("HTTP/1.1 200"));
        assert!(
            page.contains(&redirect_uri()),
            "page shows the redirect URI to register"
        );

        let bad = http_get("/start?client_id=nope");
        assert!(bad.contains("doesn't look like a Client ID"));

        let id = "0123456789abcdef0123456789abcdef";
        let start = http_get(&format!("/start?client_id={id}"));
        assert!(start.starts_with("HTTP/1.1 302"));
        let location = start
            .lines()
            .find_map(|l| l.strip_prefix("Location: "))
            .unwrap()
            .to_owned();
        assert!(location.starts_with(AUTHORIZE_URL));
        assert!(location.contains(&format!("client_id={id}")));
        let state = location
            .split('&')
            .find_map(|p| p.strip_prefix("state="))
            .unwrap()
            .to_owned();

        let forged = http_get("/callback?code=x&state=forged");
        assert!(forged.starts_with("HTTP/1.1 400"));
        assert!(
            rx.try_recv().is_err(),
            "a forged state must not complete setup"
        );

        let done = http_get(&format!("/callback?code=AUTH%2FCODE&state={state}"));
        assert!(done.contains("Done"));
        match rx.try_recv().unwrap() {
            SetupEvent::Code {
                client_id,
                code,
                verifier,
            } => {
                assert_eq!(client_id, id);
                assert_eq!(code, "AUTH/CODE");
                assert!(verifier.len() >= 43);
            }
            other => panic!("unexpected {other:?}"),
        }
        server.stop();
    }

    #[test]
    fn token_debug_hides_secrets() {
        let t = UserToken {
            client_id: "0123456789abcdef0123456789abcdef".into(),
            access_token: "SECRET_ACCESS".into(),
            refresh_token: "SECRET_REFRESH".into(),
            expires_at: 0,
        };
        let shown = format!("{t:?}");
        assert!(!shown.contains("SECRET"));
        assert!(t.is_expired());
    }

    #[test]
    fn token_round_trip() {
        let dir = std::env::temp_dir().join(format!("bigify-token-{}", std::process::id()));
        let path = dir.join("webapi-token.json");
        let t = UserToken {
            client_id: "0123456789abcdef0123456789abcdef".into(),
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: 42,
        };
        t.save(&path).unwrap();
        let loaded = UserToken::load(&path).unwrap();
        assert_eq!(loaded.refresh_token, "r");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        std::fs::remove_dir_all(dir).ok();
    }
}
