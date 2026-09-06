//! Stiki sign-on session for Pro surface unlock.
//!
//! Fail-closed: no live session unless a verified payload says `live`
//! with a subject and (if present) an unexpired timestamp. Missing file,
//! garbage JSON, and expired sessions are not signed on.
//!
//! Do not mock a paid or signed-on session. Offer-code redeem must not
//! call this module — StoreKit sheet stays ungated.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

pub const SESSION_FILE: &str = "stiki-session.json";
pub const NEED_SESSION: &str = "Pro needs a Stiki sign-on session. StoreKit alone is not enough.";

static APP_DIR: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub live: bool,
    pub subject: Option<String>,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<String>,
}

impl Session {
    pub fn none() -> Self {
        Self {
            live: false,
            subject: None,
            expires_at: None,
        }
    }
}

pub fn attach(app_dir: PathBuf) {
    let _ = APP_DIR.set(app_dir);
}

/// Fail-closed parse. `live: true` alone is not enough.
pub fn session_from_json(raw: &str) -> Session {
    let Ok(parsed) = serde_json::from_str::<Session>(raw) else {
        return Session::none();
    };
    let subject_ok = parsed
        .subject
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !parsed.live || !subject_ok {
        return Session::none();
    }
    if let Some(exp) = parsed.expires_at.as_deref() {
        if let Ok(when) = chrono::DateTime::parse_from_rfc3339(exp) {
            if when.with_timezone(&chrono::Utc) <= chrono::Utc::now() {
                return Session::none();
            }
        } else {
            return Session::none();
        }
    }
    parsed
}

pub fn current_session() -> Session {
    let Some(dir) = APP_DIR.get() else {
        return Session::none();
    };
    match fs::read_to_string(dir.join(SESSION_FILE)) {
        Ok(raw) => session_from_json(&raw),
        Err(_) => Session::none(),
    }
}

pub fn require_session() -> Result<Session, String> {
    let session = current_session();
    if session.live {
        Ok(session)
    } else {
        Err(NEED_SESSION.into())
    }
}

pub fn is_live() -> bool {
    current_session().live
}

#[tauri::command]
pub fn stiki_session() -> Session {
    current_session()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fail_closed_on_garbage() {
        assert_eq!(session_from_json(""), Session::none());
        assert_eq!(session_from_json("{"), Session::none());
        assert_eq!(session_from_json("null"), Session::none());
    }

    #[test]
    fn live_flag_alone_is_not_enough() {
        assert!(!session_from_json(r#"{"live":true}"#).live);
        assert!(!session_from_json(r#"{"live":true,"subject":""}"#).live);
        assert!(!session_from_json(r#"{"live":true,"subject":"   "}"#).live);
    }

    #[test]
    fn live_plus_subject_is_a_session() {
        let s = session_from_json(r#"{"live":true,"subject":"user@example.com"}"#);
        assert!(s.live);
        assert_eq!(s.subject.as_deref(), Some("user@example.com"));
    }

    #[test]
    fn expired_session_fails_closed() {
        let json = r#"{
            "live": true,
            "subject": "user@example.com",
            "expiresAt": "2020-01-01T00:00:00Z"
        }"#;
        assert!(!session_from_json(json).live);
    }

    #[test]
    fn unparseable_expiry_fails_closed() {
        let json = r#"{
            "live": true,
            "subject": "user@example.com",
            "expiresAt": "soon"
        }"#;
        assert!(!session_from_json(json).live);
    }

    #[test]
    fn missing_attach_is_not_signed_on() {
        assert!(!current_session().live);
        assert!(require_session().is_err());
        let err = require_session().unwrap_err();
        assert!(err.contains("Stiki"));
        assert!(err.contains("StoreKit alone"));
    }

    #[test]
    fn no_mock_sign_on_writer() {
        let src = include_str!("stiki_session.rs");
        assert!(!src.contains("fn sign_on"));
        assert!(!src.contains("fn mock_session"));
        assert!(!src.contains("pub fn grant_session"));
        assert!(src.contains("Do not mock"));
    }
}
