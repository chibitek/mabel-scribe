//! On-device org / seats / invites. Pro-gated. No network sync in v1.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::storekit;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Seat {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Invite {
    pub id: String,
    pub email: String,
    pub token: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TeamState {
    #[serde(rename = "orgName", default)]
    pub org_name: String,
    #[serde(default)]
    pub seats: Vec<Seat>,
    #[serde(default)]
    pub invites: Vec<Invite>,
}

fn path(app_dir: &PathBuf) -> PathBuf {
    app_dir.join("teams.json")
}

pub fn load(app_dir: &PathBuf) -> TeamState {
    fs::read_to_string(path(app_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(app_dir: &PathBuf, state: &TeamState) -> Result<(), String> {
    fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(path(app_dir), json).map_err(|e| e.to_string())
}

fn normalize_email(email: &str) -> Result<String, String> {
    let trimmed = email.trim().to_lowercase();
    if trimmed.is_empty() || !trimmed.contains('@') || trimmed.contains(' ') {
        return Err("Enter a valid email".into());
    }
    Ok(trimmed)
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn new_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{n:x}")
}

pub fn get(app_dir: &PathBuf) -> Result<TeamState, String> {
    storekit::require_pro()?;
    Ok(load(app_dir))
}

pub fn set_org(app_dir: &PathBuf, org_name: String) -> Result<TeamState, String> {
    storekit::require_pro()?;
    let mut state = load(app_dir);
    state.org_name = org_name.trim().to_string();
    save(app_dir, &state)?;
    Ok(state)
}

pub fn add_seat(
    app_dir: &PathBuf,
    display_name: String,
    email: String,
    role: String,
) -> Result<TeamState, String> {
    storekit::require_pro()?;
    let email = normalize_email(&email)?;
    let name = display_name.trim();
    if name.is_empty() {
        return Err("Seat needs a name".into());
    }
    let role = match role.as_str() {
        "owner" | "member" => role,
        _ => "member".into(),
    };
    let mut state = load(app_dir);
    if state.seats.iter().any(|s| s.email == email) {
        return Err("That email already has a seat".into());
    }
    state.seats.push(Seat {
        id: new_id(),
        display_name: name.to_string(),
        email,
        role,
    });
    save(app_dir, &state)?;
    Ok(state)
}

pub fn remove_seat(app_dir: &PathBuf, seat_id: String) -> Result<TeamState, String> {
    storekit::require_pro()?;
    let mut state = load(app_dir);
    let before = state.seats.len();
    state.seats.retain(|s| s.id != seat_id);
    if state.seats.len() == before {
        return Err("Seat not found".into());
    }
    save(app_dir, &state)?;
    Ok(state)
}

pub fn create_invite(app_dir: &PathBuf, email: String) -> Result<TeamState, String> {
    storekit::require_pro()?;
    let email = normalize_email(&email)?;
    let mut state = load(app_dir);
    if state
        .invites
        .iter()
        .any(|i| i.email == email && i.status == "pending")
    {
        return Err("A pending invite already exists for that email".into());
    }
    let token_src = new_id();
    state.invites.push(Invite {
        id: new_id(),
        email,
        token: format!("mabel-{}", &token_src[..12.min(token_src.len())]),
        created_at: now_iso(),
        status: "pending".into(),
    });
    save(app_dir, &state)?;
    Ok(state)
}

pub fn revoke_invite(app_dir: &PathBuf, invite_id: String) -> Result<TeamState, String> {
    storekit::require_pro()?;
    let mut state = load(app_dir);
    let Some(invite) = state.invites.iter_mut().find(|i| i.id == invite_id) else {
        return Err("Invite not found".into());
    };
    invite.status = "revoked".into();
    save(app_dir, &state)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn tmp() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "mabel-teams-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn commands_fail_closed_without_pro() {
        let dir = tmp();
        assert!(get(&dir).is_err());
        assert!(set_org(&dir, "Acme".into()).is_err());
        assert!(add_seat(&dir, "Ada".into(), "ada@example.com".into(), "owner".into()).is_err());
        assert!(create_invite(&dir, "ada@example.com".into()).is_err());
    }

    #[test]
    fn persist_roundtrip_when_called_directly() {
        let dir = tmp();
        let mut state = TeamState::default();
        state.org_name = "Chibitek".into();
        state.seats.push(Seat {
            id: "1".into(),
            display_name: "Erick".into(),
            email: "erick@example.com".into(),
            role: "owner".into(),
        });
        save(&dir, &state).unwrap();
        let loaded = load(&dir);
        assert_eq!(loaded.org_name, "Chibitek");
        assert_eq!(loaded.seats[0].email, "erick@example.com");
    }

    #[test]
    fn normalize_email_rejects_junk() {
        assert!(normalize_email("").is_err());
        assert!(normalize_email("not-an-email").is_err());
        assert!(normalize_email("a b@c.com").is_err());
        assert_eq!(
            normalize_email("  Ada@Example.com ").unwrap(),
            "ada@example.com"
        );
    }

}
