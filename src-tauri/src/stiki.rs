//! Stiki identity + KYC + Suite ACL for Mabel Connectors and team/org ACL.
//!
//! Identity is **Stiki unified login at auth.chibitek.com** — Apple, Google,
//! and Microsoft (same as Get Mochii). There is no Mabel IdP.
//!
//! Enforcer OVERRIDE RE-BOUND: Stiki required for ALL Pro unlocks.
//! StoreKit ≠ Stiki (purchase vs identity; neither substitutes).
//! Pro catalog includes Dictionary and Insights. Free dictate needs no
//! account. Clipboard Free 25 stays local. SSO still does not connect MCP.
//!
//! Fail closed. MCP is not live unless:
//! - a Stiki session exists with a Suite scope, **and**
//! - Suite ACL lists the connector, **and**
//! - Stiki KYC does not deny the scope.
//!
//! An existing Mochii Stiki session may complete Sign in with Stiki (reuse
//! Stiki in the system browser). That is identity only. MCP still needs an
//! explicit Connect + ACL after Sign in — SSO alone is never connected.
//! No cookie auto-reconnect on launch. Invalid session or ACL deny fails closed.
//!
//! Catalog (Suite ACL): `mochii` and `nexus` only. No other MCP ids.
//!
//! BREAKS IF: second IdP / Mabel-native auth; silent Stiki session enables MCP;
//! cookies auto-reconnect MCP; Free dictation gated on Stiki; Pro surface usable
//! without Stiki; sign-out leaves MCP live; login promotes local → Nexus;
//! cross-market session turns MCP on without explicit connect+ACL.

use crate::storekit;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const KYC_FILE: &str = "stiki-kyc.json";
pub const SESSION_FILE: &str = "stiki-session.json";

pub const SCOPE_MOCHII: &str = "mochii";
pub const SCOPE_NEXUS: &str = "nexus";

/// Suite ACL catalog. Anything else is denied.
pub const SUITE_SCOPES: &[&str] = &[SCOPE_MOCHII, SCOPE_NEXUS];

/// Stiki unified login host. No Mabel IdP.
pub const AUTH_HOST: &str = "auth.chibitek.com";
pub const AUTH_LOGIN_PATH: &str = "/login";
pub const AUTH_ME_PATH: &str = "/api/me";
pub const AUTH_PLATFORM: &str = "mabel";
pub const AUTH_PROVIDERS: &[&str] = &["apple", "google", "microsoft"];

pub fn auth_origin() -> String {
    format!("https://{AUTH_HOST}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KycStatus {
    Unknown,
    Allowed,
    Denied,
    Blocked,
    Expired,
}

impl KycStatus {
    pub fn is_denied(self) -> bool {
        matches!(self, Self::Denied | Self::Blocked | Self::Expired)
    }
}

impl Default for KycStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KycSnapshot {
    #[serde(default)]
    pub status: KycStatus,
    /// Optional extra deny list. Empty by default.
    #[serde(default)]
    pub denied_scopes: Vec<String>,
}

impl Default for KycSnapshot {
    fn default() -> Self {
        Self {
            status: KycStatus::Unknown,
            denied_scopes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeGrant {
    pub connector: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    #[serde(default)]
    pub signed_in: bool,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            signed_in: false,
            subject: String::new(),
            scopes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub subject: String,
    pub scopes: Vec<String>,
    pub denied: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SessionView {
    #[serde(rename = "signedIn")]
    pub signed_in: bool,
    /// Same as `signed_in`. Dictionary #17 / StoreKit UI read `live`.
    pub live: bool,
    pub subject: String,
    pub scopes: Vec<String>,
    /// True: Sign in with Stiki opens auth.chibitek.com.
    #[serde(rename = "clientWired")]
    pub client_wired: bool,
    pub hint: String,
}

/// Stiki unified login is the identity client. No Mabel IdP.
pub fn client_wired() -> bool {
    true
}

pub fn login_url(return_url: &str) -> String {
    format!(
        "{}{AUTH_LOGIN_PATH}?return_url={}&platform={AUTH_PLATFORM}",
        auth_origin(),
        url_encode(return_url)
    )
}

pub fn provider_url(provider: &str, return_url: &str) -> Result<String, String> {
    let provider = provider.trim().to_ascii_lowercase();
    if !AUTH_PROVIDERS.iter().any(|p| *p == provider) {
        return Err(format!(
            "Stiki sign-in is Apple, Google, or Microsoft only. `{provider}` is not a Stiki provider."
        ));
    }
    Ok(format!(
        "{}/auth/{provider}?return_url={}",
        auth_origin(),
        url_encode(return_url)
    ))
}

pub fn load_session(app_dir: &PathBuf) -> Session {
    // Explicit session file only. Never cookies, never auto-reconnect.
    fs::read_to_string(app_dir.join(SESSION_FILE))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Test / Suite injection only. Production sign-in never calls this
/// unless `/api/me` returned a valid identity.
pub fn write_session(app_dir: &PathBuf, session: &Session) -> Result<(), String> {
    fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    fs::write(app_dir.join(SESSION_FILE), json).map_err(|e| e.to_string())
}

pub fn clear_session(app_dir: &PathBuf) -> Result<Session, String> {
    let path = app_dir.join(SESSION_FILE);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(Session::default())
}

pub fn session_view(app_dir: &PathBuf) -> SessionView {
    let session = load_session(app_dir);
    let signed_in = require_session(app_dir).is_ok();
    let hint = if signed_in {
        "Signed in with Stiki. Pro surfaces use this session plus StoreKit. Connect Mochii MCP or Nexus MCP yourself — SSO is not connected.".into()
    } else if session.signed_in {
        "Stiki session is present but has no Suite scope. MCP stays closed.".into()
    } else {
        "Signed out. Free dictation works without an account. Pro features need Sign in with Stiki (and Activate Pro if you are not entitled).".into()
    };
    SessionView {
        signed_in,
        live: signed_in,
        subject: session.subject,
        scopes: session.scopes,
        client_wired: client_wired(),
        hint,
    }
}

/// Fail closed without a signed-in Stiki session that carries a Suite scope.
pub fn require_session(app_dir: &PathBuf) -> Result<Session, String> {
    let session = load_session(app_dir);
    if !session.signed_in {
        return Err(
            "Sign in with Stiki first. Mochii MCP and Nexus MCP stay closed.".into(),
        );
    }
    let has_suite = session.scopes.iter().any(|s| is_suite_scope(s));
    if !has_suite {
        return Err(
            "Stiki session has no Suite scope. MCP stays closed.".into(),
        );
    }
    Ok(session)
}

/// Pro unlock: StoreKit entitlement **and** a valid Stiki session.
/// Neither substitutes for the other. Free dictation does not use this.
pub fn require_pro_unlock(app_dir: &PathBuf) -> Result<Session, String> {
    storekit::require_pro()?;
    require_session(app_dir).map_err(|_| {
        "Sign in with Stiki first. Pro surfaces stay locked until a valid Stiki session exists."
            .into()
    })
}

/// UI / cap helper. True only when StoreKit Pro and Stiki session are both live.
pub fn pro_unlocked(app_dir: &PathBuf) -> bool {
    storekit::current_entitlement().entitled && require_session(app_dir).is_ok()
}

/// Custom dictionary is a Pro surface. Free dictation still runs with empty vocab.
pub fn effective_dictionary(app_dir: &PathBuf, words: &[String]) -> Vec<String> {
    if pro_unlocked(app_dir) {
        words.to_vec()
    } else {
        Vec::new()
    }
}

/// Team / org ACL door. Same Stiki session as Connectors.
pub fn require_acl(app_dir: &PathBuf) -> Result<Session, String> {
    require_session(app_dir).map_err(|_| {
        "Sign in with Stiki first. Team and org ACL stay closed until a valid Stiki session exists."
            .into()
    })
}

/// Library helper used by tests. Never invents a session.
/// Interactive Sign in with Stiki lives in the Tauri command so a click
/// is required and cookies cannot auto-reconnect.
pub fn sign_in(_app_dir: &PathBuf) -> Result<SessionView, String> {
    Err(
        "Sign in with Stiki opens auth.chibitek.com (Apple, Google, or Microsoft). Mabel does not keep a session until Stiki returns a valid identity. MCP stays closed.".into(),
    )
}

pub fn bind_callback() -> Result<(u16, TcpListener), String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener
        .set_nonblocking(false)
        .map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    Ok((port, listener))
}

pub fn wait_for_callback(listener: TcpListener, timeout: Duration) -> Result<String, String> {
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + timeout;
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let raw = read_http_request(&mut stream)?;
                let _ = write_callback_page(&mut stream);
                return Ok(raw);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(
                        "Stiki sign-in timed out. Mabel did not receive a session. MCP stays closed."
                            .into(),
                    );
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

pub fn token_from_callback(raw: &str) -> Result<String, String> {
    let target = request_target(raw);
    let query = query_of(&target);
    if let Some(token) = first_query_value(&query, &["access_token", "token", "id_token", "session"])
    {
        if !token.trim().is_empty() {
            return Ok(token);
        }
    }
    if first_query_value(&query, &["code"]).is_some() {
        return Err(
            "Stiki returned an authorization code without a session token. Fail closed — MCP stays closed.".into(),
        );
    }
    Err(
        "Stiki did not return a session token. Fail closed — MCP stays closed.".into(),
    )
}

pub fn identity_from_me_json(body: &str) -> Result<Identity, String> {
    let value: Value = serde_json::from_str(body).map_err(|_| {
        "Stiki /api/me returned an unreadable body. Fail closed.".to_string()
    })?;
    if let Some(err) = value.get("error").and_then(|v| v.as_str()) {
        return Err(format!(
            "Stiki session invalid ({err}). Fail closed — MCP stays closed."
        ));
    }
    if json_denied(&value) {
        return Err(
            "Stiki ACL denied this identity. Fail closed — Connectors and team stay closed.".into(),
        );
    }
    let subject = subject_from_me(&value).ok_or_else(|| {
        "Stiki /api/me returned no subject. Fail closed — MCP stays closed.".to_string()
    })?;
    let scopes = scopes_from_me(&value);
    Ok(Identity {
        subject,
        scopes,
        denied: false,
    })
}

/// Persist only after a valid Stiki identity. Never called from cookies.
pub fn persist_valid_identity(app_dir: &PathBuf, identity: &Identity) -> Result<Session, String> {
    if identity.denied {
        return Err(
            "Stiki ACL denied this identity. Fail closed — Connectors and team stay closed.".into(),
        );
    }
    let subject = identity.subject.trim();
    if subject.is_empty() {
        return Err("Stiki identity has no subject. Fail closed.".into());
    }
    let scopes: Vec<String> = if identity.scopes.is_empty() {
        SUITE_SCOPES.iter().map(|s| (*s).to_string()).collect()
    } else {
        identity
            .scopes
            .iter()
            .filter(|s| is_suite_scope(s))
            .cloned()
            .collect()
    };
    if scopes.is_empty() {
        return Err("Stiki session has no Suite scope. MCP stays closed.".into());
    }
    let session = Session {
        signed_in: true,
        subject: subject.to_string(),
        scopes,
    };
    write_session(app_dir, &session)?;
    Ok(session)
}

pub async fn verify_bearer(token: &str) -> Result<Identity, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("Stiki session token is empty. Fail closed.".into());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let res = client
        .get(format!("{}{AUTH_ME_PATH}", auth_origin()))
        .header("Authorization", format!("Bearer {token}"))
        .header("X-Platform", AUTH_PLATFORM)
        .send()
        .await
        .map_err(|e| format!("Stiki session could not be checked: {e}"))?;
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(identity_from_me_json(&body).err().unwrap_or_else(|| {
            "Stiki session invalid. Fail closed — MCP stays closed.".into()
        }));
    }
    identity_from_me_json(&body)
}

pub fn is_suite_scope(id: &str) -> bool {
    SUITE_SCOPES.iter().any(|s| *s == id)
}

pub fn require_catalog(id: &str) -> Result<String, String> {
    let id = id.trim().to_ascii_lowercase();
    if is_suite_scope(&id) {
        Ok(id)
    } else {
        Err(format!(
            "Stiki Suite ACL denied `{id}`. Mabel Connectors only allow Mochii MCP and Nexus MCP."
        ))
    }
}

pub fn load_kyc(app_dir: &PathBuf) -> KycSnapshot {
    let path = app_dir.join(KYC_FILE);
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist a KYC snapshot (tests / Suite drop). Production connect never
/// auto-writes this file.
pub fn write_kyc(app_dir: &PathBuf, snap: &KycSnapshot) -> Result<(), String> {
    fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(snap).map_err(|e| e.to_string())?;
    fs::write(app_dir.join(KYC_FILE), json).map_err(|e| e.to_string())
}

/// Fail closed if KYC is denied/blocked/expired or this scope is denied.
pub fn kyc_blocks(snap: &KycSnapshot, connector: &str) -> Result<(), String> {
    if snap.status.is_denied() {
        return Err(format!(
            "Stiki KYC is {:?}. Connector `{connector}` stays closed.",
            snap.status
        ));
    }
    if snap
        .denied_scopes
        .iter()
        .any(|s| s.eq_ignore_ascii_case(connector))
    {
        return Err(format!(
            "Stiki KYC denied scope `{connector}`. Connector stays closed."
        ));
    }
    Ok(())
}

/// Issue a user-initiated scope grant. Fail closed without a Stiki session,
/// on catalog miss, or denied KYC.
pub fn issue_user_grant(app_dir: &PathBuf, connector: &str) -> Result<ScopeGrant, String> {
    require_session(app_dir)?;
    let connector = require_catalog(connector)?;
    let snap = load_kyc(app_dir);
    kyc_blocks(&snap, &connector)?;
    Ok(ScopeGrant { connector })
}

/// Live MCP use requires a grant we already issued. No grant → fail closed.
pub fn require_scope(grant: Option<&ScopeGrant>, connector: &str) -> Result<(), String> {
    let connector = require_catalog(connector)?;
    match grant {
        Some(g) if g.connector == connector => Ok(()),
        _ => Err(format!(
            "Stiki scope for `{connector}` is not granted. Connect explicitly, or the connector stays closed."
        )),
    }
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn read_http_request(stream: &mut TcpStream) -> Result<String, String> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&buf[..n]).into_owned())
}

fn write_callback_page(stream: &mut TcpStream) -> Result<(), String> {
    let body = "<!doctype html><html><body style=\"font-family:sans-serif;padding:2rem\"><p>You can return to Mabel.</p></body></html>";
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(resp.as_bytes()).map_err(|e| e.to_string())
}

fn request_target(raw: &str) -> String {
    raw.lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("")
        .to_string()
}

fn query_of(target: &str) -> String {
    match target.split_once('?') {
        Some((_, q)) => q.split('#').next().unwrap_or(q).to_string(),
        None => String::new(),
    }
}

fn first_query_value(query: &str, keys: &[&str]) -> Option<String> {
    for pair in query.split('&') {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        if keys.iter().any(|want| *want == k) {
            return Some(url_decode(v));
        }
    }
    None
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = &s[i + 1..i + 3];
                if let Ok(n) = u8::from_str_radix(hex, 16) {
                    out.push(n);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn json_denied(value: &Value) -> bool {
    if value.get("denied").and_then(|v| v.as_bool()) == Some(true) {
        return true;
    }
    if value.get("blocked").and_then(|v| v.as_bool()) == Some(true) {
        return true;
    }
    if value.get("allowed").and_then(|v| v.as_bool()) == Some(false) {
        return true;
    }
    matches!(
        value.get("status").and_then(|v| v.as_str()),
        Some("denied" | "blocked" | "expired")
    )
}

fn subject_from_me(value: &Value) -> Option<String> {
    const KEYS: &[&str] = &["email", "sub", "id", "subject", "user_id"];
    for key in KEYS {
        if let Some(s) = value.get(*key).and_then(|v| v.as_str()) {
            let s = s.trim();
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    if let Some(user) = value.get("user") {
        for key in KEYS {
            if let Some(s) = user.get(*key).and_then(|v| v.as_str()) {
                let s = s.trim();
                if !s.is_empty() {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

fn scopes_from_me(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for key in ["scopes", "roles", "platforms"] {
        if let Some(arr) = value.get(key).and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(s) = item.as_str() {
                    let s = s.trim().to_ascii_lowercase();
                    if is_suite_scope(&s) && !out.contains(&s) {
                        out.push(s);
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn tmp() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "mabel-stiki-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn suite_acl_is_mochii_and_nexus_only() {
        assert_eq!(SUITE_SCOPES, &["mochii", "nexus"]);
        assert!(require_catalog("mochii").is_ok());
        assert!(require_catalog("Nexus").is_ok());
        assert!(require_catalog("slack").is_err(), "BREAKS IF: unknown connector");
        assert!(require_catalog("wispr").is_err());
        assert!(require_catalog("scratchpad").is_err());
    }

    fn signed_in_session() -> Session {
        Session {
            signed_in: true,
            subject: "test".into(),
            scopes: vec!["mochii".into(), "nexus".into()],
        }
    }

    #[test]
    fn missing_kyc_defaults_unknown_not_denied() {
        let dir = tmp();
        let snap = load_kyc(&dir);
        assert_eq!(snap.status, KycStatus::Unknown);
        assert!(kyc_blocks(&snap, "mochii").is_ok());
    }

    #[test]
    fn default_session_is_signed_out() {
        let dir = tmp();
        let session = load_session(&dir);
        assert!(!session.signed_in, "BREAKS IF: MCP without a Stiki session");
        assert!(require_session(&dir).is_err());
        assert!(client_wired());
        assert!(!session_view(&dir).signed_in);
        assert!(session_view(&dir).client_wired);
    }

    #[test]
    fn sign_in_without_callback_writes_nothing() {
        let dir = tmp();
        assert!(
            sign_in(&dir).is_err(),
            "BREAKS IF: sign-in writes a session without a valid Stiki identity"
        );
        assert!(
            !dir.join(SESSION_FILE).exists(),
            "BREAKS IF: sign-in writes a session without a valid Stiki identity"
        );
        assert!(require_session(&dir).is_err());
    }

    #[test]
    fn login_url_is_stiki_unified_not_mabel_idp() {
        let url = login_url("http://127.0.0.1:9/stiki/callback");
        assert!(url.starts_with(&format!("{}{AUTH_LOGIN_PATH}?", auth_origin())));
        assert!(url.contains("platform=mabel"));
        assert!(url.contains("return_url="));
        assert!(!url.contains("accounts.mabel"));
        assert_eq!(AUTH_PROVIDERS, &["apple", "google", "microsoft"]);
        assert!(provider_url("apple", "http://127.0.0.1/cb").is_ok());
        assert!(provider_url("google", "http://127.0.0.1/cb").is_ok());
        assert!(provider_url("microsoft", "http://127.0.0.1/cb").is_ok());
        assert!(provider_url("facebook", "http://127.0.0.1/cb").is_err());
    }

    #[test]
    fn token_from_callback_accepts_bearer_and_rejects_code_only() {
        assert_eq!(
            token_from_callback("GET /stiki/callback?access_token=abc123 HTTP/1.1").unwrap(),
            "abc123"
        );
        assert!(token_from_callback("GET /stiki/callback HTTP/1.1").is_err());
        assert!(
            token_from_callback("GET /stiki/callback?code=pkce HTTP/1.1").is_err(),
            "BREAKS IF: sign-in writes a session without a valid Stiki identity"
        );
    }

    #[test]
    fn me_json_fail_closed_on_invalid_or_acl_deny() {
        assert!(identity_from_me_json(r#"{"error":"Unauthenticated"}"#).is_err());
        assert!(identity_from_me_json(r#"{"error":"Invalid or expired token"}"#).is_err());
        assert!(identity_from_me_json(r#"{"email":"a@b.com","denied":true}"#).is_err());
        assert!(identity_from_me_json("{}").is_err());
        let ok = identity_from_me_json(r#"{"email":"ada@chibitek.com","scopes":["mochii","nexus"]}"#)
            .unwrap();
        assert_eq!(ok.subject, "ada@chibitek.com");
        assert_eq!(ok.scopes, vec!["mochii", "nexus"]);
    }

    #[test]
    fn persist_valid_identity_writes_session_invalid_does_not() {
        let dir = tmp();
        assert!(persist_valid_identity(
            &dir,
            &Identity {
                subject: String::new(),
                scopes: vec!["mochii".into()],
                denied: false,
            }
        )
        .is_err());
        assert!(!dir.join(SESSION_FILE).exists());
        persist_valid_identity(
            &dir,
            &Identity {
                subject: "ada@chibitek.com".into(),
                scopes: vec![],
                denied: false,
            },
        )
        .unwrap();
        assert!(require_session(&dir).is_ok());
        assert_eq!(load_session(&dir).subject, "ada@chibitek.com");
    }

    #[test]
    fn cookies_do_not_auto_reconnect() {
        let dir = tmp();
        fs::write(dir.join("cookies"), "stiki=1").unwrap();
        fs::write(dir.join(".stiki-cookies"), "session=1").unwrap();
        assert!(
            require_session(&dir).is_err(),
            "BREAKS IF: cookies auto-reconnect"
        );
        assert!(!session_view(&dir).signed_in);
        let src = include_str!("stiki.rs");
        assert!(src.contains("Never cookies"));
        assert!(src.contains("auth.chibitek.com"));
        assert!(src.contains("Apple"));
        let cookie_header = format!("header({q}Cookie{q}", q = '"');
        assert!(!src.contains(&cookie_header));
    }

    #[test]
    fn session_without_suite_scope_fail_closed() {
        let dir = tmp();
        write_session(
            &dir,
            &Session {
                signed_in: true,
                subject: "test".into(),
                scopes: vec!["other".into()],
            },
        )
        .unwrap();
        assert!(
            require_session(&dir).is_err(),
            "BREAKS IF: MCP without a Stiki session"
        );
        assert!(require_acl(&dir).is_err());
    }

    #[test]
    fn denied_kyc_fail_closed() {
        let dir = tmp();
        write_session(&dir, &signed_in_session()).unwrap();
        write_kyc(
            &dir,
            &KycSnapshot {
                status: KycStatus::Denied,
                denied_scopes: vec![],
            },
        )
        .unwrap();
        assert!(
            issue_user_grant(&dir, "mochii").is_err(),
            "BREAKS IF: denied KYC connects"
        );
        assert!(issue_user_grant(&dir, "nexus").is_err());
    }

    #[test]
    fn grant_without_session_fail_closed() {
        let dir = tmp();
        assert!(
            issue_user_grant(&dir, "mochii").is_err(),
            "BREAKS IF: MCP without a Stiki session"
        );
    }

    #[test]
    fn denied_scope_list_fail_closed() {
        let dir = tmp();
        write_session(&dir, &signed_in_session()).unwrap();
        write_kyc(
            &dir,
            &KycSnapshot {
                status: KycStatus::Allowed,
                denied_scopes: vec!["nexus".into()],
            },
        )
        .unwrap();
        assert!(issue_user_grant(&dir, "mochii").is_ok());
        assert!(
            issue_user_grant(&dir, "nexus").is_err(),
            "BREAKS IF: denied scope connects"
        );
    }

    #[test]
    fn require_scope_without_grant_fail_closed() {
        assert!(
            require_scope(None, "mochii").is_err(),
            "BREAKS IF: scope without grant"
        );
        let g = ScopeGrant {
            connector: "mochii".into(),
        };
        assert!(require_scope(Some(&g), "mochii").is_ok());
        assert!(
            require_scope(Some(&g), "nexus").is_err(),
            "BREAKS IF: grant reused across connectors"
        );
    }

    #[test]
    fn free_dictation_does_not_require_stiki() {
        let rec = include_str!("recorder.rs");
        assert!(
            !rec.contains("require_session"),
            "BREAKS IF: Free dictate requires account"
        );
        assert!(!rec.contains("stiki::"));
    }

    #[test]
    fn pro_unlock_requires_storekit_and_stiki() {
        let dir = tmp();
        assert!(
            require_pro_unlock(&dir).is_err(),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        write_session(&dir, &signed_in_session()).unwrap();
        assert!(
            require_pro_unlock(&dir).is_err(),
            "BREAKS IF: StoreKit==Stiki collapsed"
        );
        assert!(!pro_unlocked(&dir));
        assert!(
            effective_dictionary(&dir, &[String::from("Mabel")]).is_empty(),
            "BREAKS IF: Pro surface usable without Stiki"
        );
    }

    #[test]
    fn no_second_idp_only_stiki_apple_google_microsoft() {
        assert_eq!(AUTH_PROVIDERS, &["apple", "google", "microsoft"]);
        assert_eq!(AUTH_HOST, "auth.chibitek.com");
        assert!(provider_url("apple", "http://127.0.0.1/cb").is_ok());
        assert!(provider_url("google", "http://127.0.0.1/cb").is_ok());
        assert!(provider_url("microsoft", "http://127.0.0.1/cb").is_ok());
        assert!(
            provider_url("facebook", "http://127.0.0.1/cb").is_err(),
            "BREAKS IF: second IdP"
        );
        assert!(provider_url("mabel", "http://127.0.0.1/cb").is_err());
        assert!(provider_url("okta", "http://127.0.0.1/cb").is_err());
        let url = login_url("http://127.0.0.1/cb");
        assert!(url.contains(AUTH_HOST));
        assert!(!url.contains("accounts.google.com"));
        assert!(!url.contains("login.microsoftonline.com"));
        assert!(!url.contains("appleid.apple.com"));
        assert!(!url.contains("accounts.mabel"));
    }

    #[test]
    fn persist_identity_does_not_promote_local_to_nexus() {
        let dir = tmp();
        persist_valid_identity(
            &dir,
            &Identity {
                subject: "ada@chibitek.com".into(),
                scopes: vec!["mochii".into(), "nexus".into()],
                denied: false,
            },
        )
        .unwrap();
        assert!(
            !dir.join("connectors.json").exists(),
            "BREAKS IF: silent Stiki→MCP"
        );
        assert!(
            !dir.join("scratchpad.txt").exists(),
            "BREAKS IF: login promotes local → Nexus"
        );
        assert!(
            !dir.join("clipboard-history.json").exists(),
            "BREAKS IF: login promotes local → Nexus"
        );
        let persist = include_str!("stiki.rs")
            .split("pub fn persist_valid_identity")
            .nth(1)
            .unwrap();
        let persist = persist.split("pub async fn verify_bearer").next().unwrap();
        assert!(
            !persist.contains("connectors"),
            "BREAKS IF: silent Stiki→MCP"
        );
        assert!(
            !persist.contains("scratchpad"),
            "BREAKS IF: login promotes local → Nexus"
        );
        assert!(
            !persist.contains("captures_write"),
            "BREAKS IF: login promotes local → Nexus"
        );
    }

    #[test]
    fn team_acl_requires_stiki_session() {
        let dir = tmp();
        assert!(
            require_acl(&dir).is_err(),
            "BREAKS IF: MCP without a Stiki session"
        );
        write_session(&dir, &signed_in_session()).unwrap();
        assert!(require_acl(&dir).is_ok());
    }
}
