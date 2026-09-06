//! Product **Connectors** + Enforcer Sign-on BOUND (Suite b6530197).
//!
//! GREEN
//! - Settings→Account: Sign in with Stiki only (Apple/Google/Microsoft → Stiki
//!   unified, Mochii-same KYC)
//! - Cross-market Mochii/Stiki session OK for Mabel Account identity (no second IdP)
//! - Stiki before Mochii/Nexus MCP; MCP still needs explicit connect + Stiki ACL
//!   fail closed (SSO/session alone ≠ MCP on)
//! - Sign-out disconnects MCP immediately
//! - Free dictation: no account (works offline without Stiki)
//! - Clipboard Free 25 local slots: no account (if History opt-in)
//! - Pro unlock = StoreKit + Stiki session. Catalog: Dictionary, Polish,
//!   Clipboard Pro unlimited, Snippets, Style, Transforms, Scratchpad,
//!   Insights, Connectors/team, Teams
//! - Sign-out locks Pro immediately; Free dictate + Free clipboard stay
//! - Login/session does NOT promote Scratchpad/dictation/history/Polish/clipboard
//!   to Nexus
//! - No silent Stiki bootstrap; no always-on cookies auto-reconnect MCP without
//!   explicit connect
//! - No HIPAA/BAA claim
//!
//! Enforcer OVERRIDE RE-BOUND (Suite b6530197): Stiki required for ALL Pro
//! unlocks — not Connectors-only. StoreKit ≠ Stiki (neither substitutes).
//!
//! BREAKS IF: second IdP / Mabel-native auth; silent Stiki session enables MCP;
//! cookies auto-reconnect MCP; Free dictation gated on Stiki; Pro surface
//! usable without Stiki; sign-out leaves MCP live; login promotes local → Nexus;
//! cross-market session turns MCP on without explicit connect+ACL.
//!
//! Product LOCK still applies: Pro only. Free → Plans. Cat UI. No Wispr brand.
//! Disconnect anytime. Scratchpad is not an MCP source/sink in v1.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::stiki;
use crate::storekit;

/// Named Enforcer BOUND. `enforcer_bound_*` tests fail if this is violated.
pub const ENFORCER_BOUND: &str = "explicit user connect Mochii+Nexus MCP only; Sign in with Stiki before MCP; Stiki KYC + Suite ACL fail closed; Scratchpad NOT MCP source/sink v1; Scratchpad v1 CONFIRMED local-only; Insights NOT MCP source/sink v1; Insights v1 CONFIRMED local-only; no cloud/team/Nexus/Mochii auto-push; no silent ASR/Polish/Scratchpad/history/clipboard auto-push; no default always-on; no cookie auto-reconnect; no HIPAA/BAA; no silent Stiki bootstrap; cross-market Stiki session is identity only; SSO ≠ MCP connected; Stiki required for ALL Pro unlocks; not Connectors-only; StoreKit ≠ Stiki; Dictionary and Insights require Stiki + StoreKit; Style requires Stiki + StoreKit; Transforms requires Stiki + StoreKit; Scratchpad requires Stiki + StoreKit; Insights requires Stiki + StoreKit; Free dictation no Stiki; Clipboard Free 25 no Stiki";

/// Suite id for the canonical Sign-on BOUND tip.
pub const SIGN_ON_SUITE: &str = "b6530197";

/// Canonical Enforcer Sign-on BOUND (Suite b6530197). `enforcer_bound_sign_on_suite_*` tests fold this.
pub const SIGN_ON_BOUND: &str = "GREEN: Settings→Account Sign in with Stiki only (Apple/Google/Microsoft → Stiki unified, Mochii-same KYC); cross-market Mochii/Stiki session OK for Mabel Account identity (no second IdP); Stiki required for ALL Pro unlocks (Polish/Dictionary/Snippets/Style/Scratchpad Pro/Transforms/Insights/Connectors/Teams) — not Connectors-only; StoreKit ≠ Stiki (purchase vs identity; neither substitutes); Stiki before Mochii/Nexus MCP; MCP still needs explicit connect + Stiki ACL fail closed (SSO/session alone ≠ MCP on); Connectors: explicit connect + ACL after SSO (SSO ≠ MCP on); sign-out disconnects MCP immediately; sign-out drops MCP; Pro locks until Stiki again; sign-out locks Pro surfaces immediately; Free dictate without account OK; Free dictation no account; Clipboard Free 25 no Stiki; Pro unlock = StoreKit + Stiki session; Pro surfaces require Stiki session + StoreKit; Pro catalog: Dictionary, Polish, Clipboard Pro unlimited, Snippets, Style, Transforms, Scratchpad, Insights, Connectors/team, Teams; login/session does NOT promote Scratchpad/dictation/history/Polish/clipboard to Nexus; no silent Stiki bootstrap; no always-on cookies auto-reconnect MCP without explicit connect; no HIPAA/BAA claim; BREAKS IF: Pro unlocks without Stiki; Pro surface usable without Stiki; Free dictate requires account; Free dictation gated on Stiki; StoreKit==Stiki collapsed; login promotes local→Nexus; SSO alone enables MCP; second IdP / Mabel-native auth parallel to Stiki; silent Stiki session enables MCP; always-on cookies auto-reconnect MCP without explicit connect; sign-out leaves MCP live; login/session promotes Scratchpad/dictation/history/Polish/clipboard → Nexus; cross-market session silently turns MCP on without explicit connect+ACL";

/// Enforcer OVERRIDE RE-BOUND (Suite b6530197). Stiki for ALL Pro unlocks.
pub const ERICK_OVERRIDE: &str = "Stiki required for ALL Pro unlocks; not Connectors-only; StoreKit ≠ Stiki; neither substitutes; Pro unlock = StoreKit + Stiki session; Pro catalog: Dictionary, Polish, Clipboard Pro unlimited, Snippets, Style, Transforms, Scratchpad, Insights, Connectors/team, Teams; Free dictate without account OK; Free dictation no account; Clipboard Free 25 no Stiki; sign-out drops MCP; Pro locks until Stiki again; sign-out locks Pro immediately; SSO ≠ MCP on; no silent promote local→Nexus; no second IdP; no HIPAA/BAA";

/// Sign-on addendum (PASS). Cross-market Stiki = identity. Connect + ACL still required.
pub const SIGN_ON_ADDENDUM: &str =
    "cross-market Stiki session (Get Mochii) is identity only; MCP requires explicit Connect + ACL after Sign in; SSO alone is never connected";

pub const STORE_FILE: &str = "connectors.json";

pub const ID_MOCHII: &str = "mochii";
pub const ID_NEXUS: &str = "nexus";

pub const CATALOG: &[&str] = &[ID_MOCHII, ID_NEXUS];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectorRecord {
    #[serde(default)]
    pub connected: bool,
}

impl Default for ConnectorRecord {
    fn default() -> Self {
        Self { connected: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectorStore {
    #[serde(default)]
    pub mochii: ConnectorRecord,
    #[serde(default)]
    pub nexus: ConnectorRecord,
}

impl Default for ConnectorStore {
    fn default() -> Self {
        Self {
            mochii: ConnectorRecord::default(),
            nexus: ConnectorRecord::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ConnectorView {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub connected: bool,
    pub live: bool,
    pub hint: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ConnectorsStatus {
    pub entitled: bool,
    /// Stiki identity only. Not StoreKit Pro.
    #[serde(rename = "signedIn")]
    pub signed_in: bool,
    #[serde(rename = "stikiHint")]
    pub stiki_hint: String,
    #[serde(rename = "stikiClientWired")]
    pub stiki_client_wired: bool,
    pub catalog: Vec<ConnectorView>,
    /// Always false in v1. Scratchpad is local-only.
    #[serde(rename = "scratchpadIsMcpSource")]
    pub scratchpad_is_mcp_source: bool,
    /// Always false in v1. Scratchpad is local-only.
    #[serde(rename = "scratchpadIsMcpSink")]
    pub scratchpad_is_mcp_sink: bool,
    /// Always false. Connectors never default on.
    #[serde(rename = "defaultAlwaysOn")]
    pub default_always_on: bool,
}

fn display_name(id: &str) -> &'static str {
    match id {
        ID_MOCHII => "Mochii MCP",
        ID_NEXUS => "Nexus MCP",
        _ => "Unknown",
    }
}

fn hint_for(id: &str, connected: bool, live: bool, signed_in: bool) -> String {
    if live {
        format!(
            "{} is connected. Mabel still will not auto-push dictation, Polish, Scratchpad, or clipboard. Disconnect anytime.",
            display_name(id)
        )
    } else if !signed_in {
        format!(
            "Sign in with Stiki in Settings → Account before connecting {}.",
            display_name(id)
        )
    } else if connected {
        format!(
            "{} is marked connected but Stiki scope is not live. Fail closed — no MCP calls.",
            display_name(id)
        )
    } else {
        format!(
            "Signed in. Connect {} yourself. A Stiki session is identity only — SSO is not a connection.",
            display_name(id)
        )
    }
}

fn read_store(app_dir: &PathBuf) -> ConnectorStore {
    fs::read_to_string(app_dir.join(STORE_FILE))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_store(app_dir: &PathBuf, store: &ConnectorStore) -> Result<(), String> {
    fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(app_dir.join(STORE_FILE), json).map_err(|e| e.to_string())
}

fn record_mut<'a>(store: &'a mut ConnectorStore, id: &str) -> Result<&'a mut ConnectorRecord, String> {
    match id {
        ID_MOCHII => Ok(&mut store.mochii),
        ID_NEXUS => Ok(&mut store.nexus),
        other => Err(format!(
            "Unknown connector `{other}`. Catalog is Mochii MCP and Nexus MCP only."
        )),
    }
}

fn record_of<'a>(store: &'a ConnectorStore, id: &str) -> &'a ConnectorRecord {
    match id {
        ID_MOCHII => &store.mochii,
        ID_NEXUS => &store.nexus,
        _ => unreachable!("catalog ids only"),
    }
}

fn grant_for(store: &ConnectorStore, id: &str) -> Option<stiki::ScopeGrant> {
    let rec = record_of(store, id);
    if rec.connected {
        Some(stiki::ScopeGrant {
            connector: id.to_string(),
        })
    } else {
        None
    }
}

/// Drop leftover connected flags. Sign-in never treats SSO as connected.
fn keep_doors_disconnected(app_dir: &PathBuf) -> Result<(), String> {
    let mut store = read_store(app_dir);
    store.mochii.connected = false;
    store.nexus.connected = false;
    write_store(app_dir, &store)
}

/// Accept a Stiki identity (including a cross-market Get Mochii session).
/// Persists identity and forces MCP disconnected until explicit Connect + ACL.
pub fn accept_identity(
    app_dir: &PathBuf,
    identity: &stiki::Identity,
) -> Result<ConnectorsStatus, String> {
    stiki::persist_valid_identity(app_dir, identity)?;
    keep_doors_disconnected(app_dir)?;
    Ok(status(app_dir))
}

/// Persist-time connect. Pro + Stiki session + Suite catalog + KYC.
/// Sign-in is identity/ACL only — it does not grant Pro and does not connect.
pub fn connect(app_dir: &PathBuf, id: &str) -> Result<ConnectorsStatus, String> {
    storekit::require_pro()?;
    stiki::require_session(app_dir)?;
    let id = stiki::require_catalog(id)?;
    let _grant = stiki::issue_user_grant(app_dir, &id)?;
    let mut store = read_store(app_dir);
    record_mut(&mut store, &id)?.connected = true;
    write_store(app_dir, &store)?;
    Ok(status(app_dir))
}

/// Disconnect anytime. Allowed without Pro so a lapsed seat can close the door.
pub fn disconnect(app_dir: &PathBuf, id: &str) -> Result<ConnectorsStatus, String> {
    let id = stiki::require_catalog(id)?;
    let mut store = read_store(app_dir);
    record_mut(&mut store, &id)?.connected = false;
    write_store(app_dir, &store)?;
    Ok(status(app_dir))
}

/// Sign-out clears the Stiki session and disconnects every MCP door.
pub fn sign_out(app_dir: &PathBuf) -> Result<ConnectorsStatus, String> {
    stiki::clear_session(app_dir)?;
    let mut store = read_store(app_dir);
    store.mochii.connected = false;
    store.nexus.connected = false;
    write_store(app_dir, &store)?;
    Ok(status(app_dir))
}

pub fn status(app_dir: &PathBuf) -> ConnectorsStatus {
    let entitled = storekit::current_entitlement().entitled;
    let session = stiki::session_view(app_dir);
    let signed_in = session.signed_in;
    let store = read_store(app_dir);
    let kyc = stiki::load_kyc(app_dir);
    let catalog = CATALOG
        .iter()
        .map(|id| {
            let connected = record_of(&store, id).connected;
            let live = entitled
                && signed_in
                && connected
                && stiki::kyc_blocks(&kyc, id).is_ok()
                && stiki::require_scope(grant_for(&store, id).as_ref(), id).is_ok();
            ConnectorView {
                id: (*id).to_string(),
                name: display_name(id).to_string(),
                kind: "mcp".into(),
                connected,
                live,
                hint: hint_for(id, connected, live, signed_in),
            }
        })
        .collect();
    ConnectorsStatus {
        entitled,
        signed_in,
        stiki_hint: session.hint,
        stiki_client_wired: session.client_wired,
        catalog,
        scratchpad_is_mcp_source: false,
        scratchpad_is_mcp_sink: false,
        default_always_on: false,
    }
}

/// Live MCP use. Fail closed without Pro + Stiki session + explicit connect.
pub fn require_live(app_dir: &PathBuf, id: &str) -> Result<String, String> {
    storekit::require_pro()?;
    stiki::require_session(app_dir)?;
    let id = stiki::require_catalog(id)?;
    let store = read_store(app_dir);
    if !record_of(&store, &id).connected {
        return Err(format!(
            "{} is not connected. Connect it yourself — Mabel does not auto-connect.",
            display_name(&id)
        ));
    }
    stiki::kyc_blocks(&stiki::load_kyc(app_dir), &id)?;
    stiki::require_scope(grant_for(&store, &id).as_ref(), &id)?;
    Ok(id)
}

/// v1: there is no dictation / ASR / Polish / clipboard / history push.
pub fn auto_push_forbidden(source: &str) -> Result<(), String> {
    Err(format!(
        "v1 forbids silent MCP push from `{source}`. Connectors never auto-send ASR, Polish, Scratchpad, history, or clipboard."
    ))
}

/// v1: Scratchpad is not an MCP source.
pub fn scratchpad_as_mcp_source(_text: &str) -> Result<(), String> {
    Err("Scratchpad stays on this Mac in v1. It is not a Mochii or Nexus MCP source.".into())
}

/// v1: Scratchpad is not an MCP sink.
pub fn scratchpad_as_mcp_sink(_text: &str) -> Result<(), String> {
    Err("Scratchpad stays on this Mac in v1. It is not a Mochii or Nexus MCP sink.".into())
}

/// Later Make It So — user-initiated Scratchpad → Captures/Nexus. Stub.
pub fn export_scratchpad_to_captures(_text: &str) -> Result<(), String> {
    Err("Scratchpad export to Captures/Nexus is not available in v1. Notes stay on this Mac.".into())
}

/// v1: Insights is not an MCP source.
pub fn insights_as_mcp_source(_summary: &str) -> Result<(), String> {
    Err("Insights stays on this Mac in v1. It is not a Mochii or Nexus MCP source.".into())
}

/// v1: Insights is not an MCP sink.
pub fn insights_as_mcp_sink(_summary: &str) -> Result<(), String> {
    Err("Insights stays on this Mac in v1. It is not a Mochii or Nexus MCP sink.".into())
}

/// Later Make It So — user-initiated Insights → Captures/Nexus. Stub.
pub fn export_insights_to_captures(_summary: &str) -> Result<(), String> {
    Err("Insights export to Captures/Nexus is not available in v1. Counts stay on this Mac.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn tmp() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "mabel-connectors-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn default_is_disconnected() {
        let dir = tmp();
        let s = status(&dir);
        assert!(!s.default_always_on, "BREAKS IF: default always-on");
        assert!(!s.scratchpad_is_mcp_source, "BREAKS IF: Scratchpad auto-MCP'd");
        assert!(!s.scratchpad_is_mcp_sink, "BREAKS IF: Scratchpad auto-MCP'd");
        assert_eq!(s.catalog.len(), 2);
        for c in &s.catalog {
            assert!(!c.connected, "BREAKS IF: default always-on");
            assert!(!c.live, "BREAKS IF: default always-on");
            assert!(c.kind == "mcp");
        }
        assert_eq!(s.catalog[0].id, ID_MOCHII);
        assert_eq!(s.catalog[1].id, ID_NEXUS);
        assert!(!s.signed_in, "BREAKS IF: MCP without a Stiki session");
        assert!(s.stiki_client_wired);
        assert!(s.stiki_hint.contains("Sign in with Stiki"));
    }

    fn inject_session(dir: &PathBuf) {
        stiki::write_session(
            dir,
            &stiki::Session {
                signed_in: true,
                subject: "test".into(),
                scopes: vec!["mochii".into(), "nexus".into()],
            },
        )
        .unwrap();
    }

    #[test]
    fn mcp_connect_fails_closed_without_stiki_session() {
        let dir = tmp();
        assert!(
            connect(&dir, ID_MOCHII).is_err(),
            "BREAKS IF: MCP without a Stiki session"
        );
        inject_session(&dir);
        // Session is not Pro. StoreKit stays the entitlement gate.
        assert!(
            connect(&dir, ID_MOCHII).is_err(),
            "BREAKS IF: Stiki session conflated with StoreKit Pro"
        );
        assert!(status(&dir).signed_in);
        assert!(!status(&dir).entitled);
        assert!(status(&dir).catalog.iter().all(|c| !c.live));
    }

    #[test]
    fn sign_out_drops_connectors() {
        let dir = tmp();
        inject_session(&dir);
        let mut store = ConnectorStore::default();
        store.mochii.connected = true;
        store.nexus.connected = true;
        write_store(&dir, &store).unwrap();
        assert!(status(&dir).signed_in);
        let s = sign_out(&dir).unwrap();
        assert!(!s.signed_in, "BREAKS IF: MCP without a Stiki session");
        assert!(s.catalog.iter().all(|c| !c.connected && !c.live));
        assert!(stiki::require_session(&dir).is_err());
        assert!(!dir.join(stiki::SESSION_FILE).exists());
    }

    #[test]
    fn catalog_is_mochii_and_nexus_only() {
        assert_eq!(CATALOG, &["mochii", "nexus"]);
        let dir = tmp();
        assert!(
            connect(&dir, "slack").is_err(),
            "BREAKS IF: connectors without explicit connect + ACL"
        );
        assert!(connect(&dir, "wispr").is_err());
        assert!(connect(&dir, "scratchpad").is_err());
        assert!(disconnect(&dir, "notion").is_err());
    }

    #[test]
    fn free_cannot_connect() {
        let dir = tmp();
        assert!(
            connect(&dir, ID_MOCHII).is_err(),
            "BREAKS IF: connectors without explicit connect + ACL"
        );
        assert!(connect(&dir, ID_NEXUS).is_err());
        assert!(require_live(&dir, ID_MOCHII).is_err());
        let s = status(&dir);
        assert!(!s.entitled);
        assert!(s.catalog.iter().all(|c| !c.connected && !c.live));
    }

    #[test]
    fn disconnect_without_pro_still_closes() {
        let dir = tmp();
        let mut store = ConnectorStore::default();
        store.mochii.connected = true;
        write_store(&dir, &store).unwrap();
        let s = disconnect(&dir, ID_MOCHII).unwrap();
        assert!(!s.catalog.iter().any(|c| c.id == ID_MOCHII && c.connected));
    }

    #[test]
    fn denied_kyc_cannot_connect() {
        let dir = tmp();
        stiki::write_kyc(
            &dir,
            &stiki::KycSnapshot {
                status: stiki::KycStatus::Denied,
                denied_scopes: vec![],
            },
        )
        .unwrap();
        assert!(
            connect(&dir, ID_NEXUS).is_err(),
            "BREAKS IF: connectors without explicit connect + ACL"
        );
    }

    #[test]
    fn silent_push_and_scratchpad_mcp_fail_closed() {
        assert!(
            auto_push_forbidden("asr").is_err(),
            "BREAKS IF: silent push to Nexus/Mochii"
        );
        assert!(auto_push_forbidden("polish").is_err());
        assert!(auto_push_forbidden("scratchpad").is_err());
        assert!(auto_push_forbidden("insights").is_err());
        assert!(auto_push_forbidden("clipboard").is_err());
        assert!(auto_push_forbidden("history").is_err());
        assert!(
            scratchpad_as_mcp_source("notes").is_err(),
            "BREAKS IF: Scratchpad auto-MCP'd"
        );
        assert!(scratchpad_as_mcp_sink("notes").is_err());
        assert!(export_scratchpad_to_captures("notes").is_err());
        assert!(
            insights_as_mcp_source("counts").is_err(),
            "BREAKS IF: Insights auto-MCP'd"
        );
        assert!(insights_as_mcp_sink("counts").is_err());
        assert!(export_insights_to_captures("counts").is_err());
    }

    #[test]
    fn recorder_polish_scratchpad_clipboard_do_not_auto_push() {
        let rec = include_str!("recorder.rs");
        assert!(
            !rec.contains("connectors::"),
            "BREAKS IF: silent push to Nexus/Mochii"
        );
        assert!(!rec.contains("auto_push_forbidden"));
        assert!(!rec.contains("scratchpad_as_mcp_source"));
        let stream = include_str!("streaming.rs");
        assert!(!stream.contains("connectors::"), "BREAKS IF: silent push");
        let polish = include_str!("polish.rs");
        assert!(!polish.contains("connectors::connect"), "BREAKS IF: silent push");
        let llm = include_str!("llm.rs");
        let polish_fn = llm.split("pub async fn polish_or_rules").nth(1).unwrap();
        let polish_fn = polish_fn.split("pub async fn ensure_and_cleanup").next().unwrap();
        assert!(!polish_fn.contains("connectors"), "BREAKS IF: silent push");
        let pad = include_str!("pro_features.rs");
        let get = pad.split("pub fn scratchpad_read").nth(1).unwrap();
        let get = get.split("pub fn scratchpad_write_local").next().unwrap();
        assert!(!get.contains("connectors"), "BREAKS IF: Scratchpad auto-MCP'd");
        let save = pad.split("pub fn scratchpad_write_local").nth(1).unwrap();
        let save = save.split("#[cfg(test)]").next().unwrap();
        assert!(!save.contains("connectors"), "BREAKS IF: Scratchpad auto-MCP'd");
        assert!(!save.contains("captures_write"));
        let product = include_str!("scratchpad.rs");
        let product_save = product.split("pub fn save").nth(1).unwrap();
        let product_save = product_save.split("pub fn clear").next().unwrap();
        assert!(!product_save.contains("connectors"), "BREAKS IF: Scratchpad auto-MCP'd");
        let clip = include_str!("clipboard_history.rs");
        assert!(!clip.contains("connectors::"), "BREAKS IF: silent push");
        let main = include_str!("main.rs");
        let scratch_save = main.split("fn scratchpad_save").nth(1).unwrap();
        let scratch_save = scratch_save.split("fn connectors_status").next().unwrap();
        assert!(
            !scratch_save.contains("connectors::"),
            "BREAKS IF: Scratchpad auto-MCP'd"
        );
        assert!(
            scratch_save.contains("local-only"),
            "BREAKS IF: Scratchpad auto-MCP'd"
        );
    }

    #[test]
    fn enforcer_bound_breaks_if_auto_mcp_silent_push_or_hipaa() {
        assert!(ENFORCER_BOUND.contains("explicit user connect"));
        assert!(ENFORCER_BOUND.contains("Mochii+Nexus MCP only"));
        assert!(ENFORCER_BOUND.contains("Sign in with Stiki before MCP"));
        assert!(ENFORCER_BOUND.contains("Stiki KYC + Suite ACL fail closed"));
        assert!(ENFORCER_BOUND.contains("Scratchpad NOT MCP source/sink v1"));
        assert!(ENFORCER_BOUND.contains("Scratchpad v1 CONFIRMED local-only"));
        assert!(ENFORCER_BOUND.contains("Insights NOT MCP source/sink v1"));
        assert!(ENFORCER_BOUND.contains("Insights v1 CONFIRMED local-only"));
        assert!(ENFORCER_BOUND.contains("no cloud/team/Nexus/Mochii auto-push"));
        assert!(ENFORCER_BOUND.contains("no silent ASR/Polish/Scratchpad/history/clipboard auto-push"));
        assert!(ENFORCER_BOUND.contains("no default always-on"));
        assert!(ENFORCER_BOUND.contains("no cookie auto-reconnect"));
        assert!(ENFORCER_BOUND.contains("no HIPAA/BAA"));
        assert!(ENFORCER_BOUND.contains("cross-market Stiki session is identity only"));
        assert!(ENFORCER_BOUND.contains("SSO ≠ MCP connected"));
        assert!(ENFORCER_BOUND.contains("no silent Stiki bootstrap"));
        assert!(ENFORCER_BOUND.contains("Stiki required for ALL Pro unlocks"));
        assert!(ENFORCER_BOUND.contains("not Connectors-only"));
        assert!(ENFORCER_BOUND.contains("StoreKit ≠ Stiki"));
        assert!(ENFORCER_BOUND.contains("Dictionary and Insights require Stiki + StoreKit"));
        assert!(ENFORCER_BOUND.contains("Style requires Stiki + StoreKit"));
        assert!(ENFORCER_BOUND.contains("Transforms requires Stiki + StoreKit"));
        assert!(ENFORCER_BOUND.contains("Scratchpad requires Stiki + StoreKit"));
        assert!(ENFORCER_BOUND.contains("Insights requires Stiki + StoreKit"));
        assert!(ENFORCER_BOUND.contains("Free dictation no Stiki"));
        assert!(ENFORCER_BOUND.contains("Clipboard Free 25 no Stiki"));

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        let help = html.to_ascii_lowercase();
        assert!(
            !help.contains("hipaa") && !help.contains("baa"),
            "BREAKS IF: HIPAA/BAA copy"
        );
        assert!(!ts.to_ascii_lowercase().contains("hipaa"), "BREAKS IF: HIPAA/BAA copy");
        assert!(!html.to_ascii_lowercase().contains("wispr"));
        assert!(!ts.to_ascii_lowercase().contains("wispr"));
        assert!(html.contains("data-view=\"connectors\""));
        assert!(html.contains("id=\"connector-mochii\""));
        assert!(html.contains("id=\"connector-nexus\""));
        assert!(html.contains("id=\"connectors-activate\""));
        assert!(html.contains("id=\"stiki-signin\""));
        assert!(html.contains("Sign in with Stiki"));
        assert!(html.contains("Apple"));
        assert!(html.contains("Google"));
        assert!(html.contains("Microsoft"));
        assert!(html.contains("auth.chibitek.com"));
        assert!(html.contains("data-view=\"dictionary\""));
        assert!(html.contains("data-view=\"insights\""));
        assert!(html.contains("id=\"stiki-signout\""));
        assert!(html.contains("Disconnect"));
        assert!(html.contains("Connect"));
        assert!(html.contains("Scratchpad stays on this Mac"));
        assert!(!html.contains("id=\"scratchpad-export-nexus\""));
        assert!(!html.contains("id=\"scratchpad-export-mochii\""));
        assert!(ts.contains("connectors_connect"));
        assert!(ts.contains("connectors_disconnect"));
        assert!(ts.contains("stiki_sign_in"));
        assert!(ts.contains("stiki_sign_out"));
        assert!(ts.contains("openPlans()"));
        assert!(ts.contains("openAccount"));
        assert!(!ts.contains("https://chibiteklabs"));

        let main = include_str!("main.rs");
        assert!(main.contains("connectors_connect"));
        assert!(main.contains("connectors_disconnect"));
        assert!(main.contains("connectors::connect"));
        assert!(main.contains("stiki_sign_in"));
        assert!(main.contains("stiki_sign_out"));
        assert!(main.contains("connectors::sign_out"));
        assert!(main.contains("auth.chibitek.com") || include_str!("stiki.rs").contains("auth.chibitek.com"));
        assert!(include_str!("stiki.rs").contains("verify_bearer"));
        assert!(include_str!("stiki.rs").contains("persist_valid_identity"));
        assert!(!main.contains("MabelSpatial"));

        let settings = crate::settings::Settings::default();
        assert_eq!(settings.polish_mode, "off");
        assert!(!settings.clipboard_history_enabled);

        let storekit = include_str!("storekit.rs");
        assert!(
            !storekit.contains("stiki-session"),
            "BREAKS IF: Stiki session conflated with StoreKit Pro"
        );
        assert!(!storekit.contains("signed_in"));
    }

    #[test]
    fn local_features_work_without_stiki() {
        let rec = include_str!("recorder.rs");
        assert!(!rec.contains("require_session"), "BREAKS IF: Free dictate requires account");
        assert!(!rec.contains("require_pro_unlock"));
        assert!(!rec.contains("stiki_sign"));
        let polish = include_str!("polish.rs");
        assert!(
            polish.contains("require_pro_unlock"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        let clip = include_str!("clipboard_history.rs");
        assert!(
            clip.contains("pro_unlocked"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        assert!(clip.contains("FREE_CAP"));
        let pad = include_str!("scratchpad.rs");
        assert!(
            pad.contains("require_surface") && pad.contains("stiki_session"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        let ins = include_str!("insights.rs");
        assert!(
            ins.contains("require_surface") && ins.contains("stiki_session"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        let settings = include_str!("settings.rs");
        assert!(!settings.contains("stiki-session"));
        assert!(!settings.contains("signedIn"));
        let insights = include_str!("stats.rs");
        assert!(!insights.contains("require_session"), "BREAKS IF: Free dictate requires account");
        assert!(
            !insights.contains("require_pro_unlock"),
            "BREAKS IF: Free dictate requires account"
        );
        let main = include_str!("main.rs");
        let stats_cmd = main.split("fn get_stats").nth(1).unwrap();
        let stats_cmd = stats_cmd.split("fn set_launch_at_login").next().unwrap();
        assert!(
            stats_cmd.contains("insights::require_summary") || stats_cmd.contains("require_pro_unlock"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        let insights_cmd = main.split("fn insights_get").nth(1).expect("insights_get");
        let insights_cmd = insights_cmd.split("fn ").next().unwrap();
        assert!(
            insights_cmd.contains("insights::require_summary"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        let ts = include_str!("../../src/main.ts");
        let persist = ts.split("function renderDictionary").nth(1);
        if let Some(persist) = persist {
            let persist = persist.split("async function persistPolishMode").next().unwrap_or(persist);
            assert!(!persist.contains("stiki_sign"));
            assert!(!persist.contains("connectors_connect"));
        }
    }

    #[test]
    fn enforcer_bound_sign_on_suite_b6530197() {
        assert_eq!(SIGN_ON_SUITE, "b6530197");
        // Canonical GREEN tip.
        assert!(SIGN_ON_BOUND.contains("Settings→Account Sign in with Stiki only"));
        assert!(SIGN_ON_BOUND.contains("Apple/Google/Microsoft → Stiki unified"));
        assert!(SIGN_ON_BOUND.contains("Mochii-same KYC"));
        assert!(SIGN_ON_BOUND.contains("cross-market Mochii/Stiki session OK for Mabel Account identity"));
        assert!(SIGN_ON_BOUND.contains("no second IdP"));
        assert!(SIGN_ON_BOUND.contains("Stiki before Mochii/Nexus MCP"));
        assert!(SIGN_ON_BOUND.contains("explicit connect + Stiki ACL fail closed"));
        assert!(SIGN_ON_BOUND.contains("SSO/session alone ≠ MCP on"));
        assert!(SIGN_ON_BOUND.contains("sign-out disconnects MCP immediately"));
        assert!(SIGN_ON_BOUND.contains("Stiki required for ALL Pro unlocks"));
        assert!(SIGN_ON_BOUND.contains("Snippets/Style"));
        assert!(SIGN_ON_BOUND.contains("Style/Scratchpad Pro/Transforms"));
        assert!(include_str!("scratchpad.rs").contains("CONFIRMED Suite b6530197"));
        assert!(include_str!("scratchpad.rs").contains("enforcer_bound_scratchpad_v1_suite_b6530197"));
        assert!(include_str!("scratchpad.rs").contains("no cloud/team/Nexus/Mochii auto-push"));
        assert!(include_str!("insights.rs").contains("CONFIRMED Suite b6530197"));
        assert!(include_str!("insights.rs").contains("enforcer_bound_insights_v1_suite_b6530197"));
        assert!(include_str!("insights.rs").contains("Insights NOT MCP source/sink v1"));
        assert!(include_str!("insights.rs").contains("Insights v1 CONFIRMED local-only"));
        assert!(include_str!("insights.rs").contains("no cloud/team/Nexus/Mochii auto-push"));
        assert!(include_str!("insights.rs").contains("BREAKS IF: without dual gate"));
        assert!(include_str!("insights.rs").contains("BREAKS IF: cloud analytics"));
        assert!(include_str!("insights.rs").contains("BREAKS IF: cross-device sync"));
        assert!(include_str!("insights.rs").contains("BREAKS IF: Nexus write"));
        assert!(include_str!("insights.rs").contains("BREAKS IF: Free dictate gated"));
        assert!(include_str!("insights.rs").contains("BREAKS IF: HIPAA"));
        assert!(SIGN_ON_BOUND.contains("not Connectors-only"));
        assert!(SIGN_ON_BOUND.contains("StoreKit ≠ Stiki"));
        assert!(SIGN_ON_BOUND.contains("neither substitutes"));
        assert!(SIGN_ON_BOUND.contains("Free dictate without account OK"));
        assert!(SIGN_ON_BOUND.contains("Free dictation no account"));
        assert!(SIGN_ON_BOUND.contains("Clipboard Free 25 no Stiki"));
        assert!(SIGN_ON_BOUND.contains("explicit connect + ACL after SSO"));
        assert!(SIGN_ON_BOUND.contains("sign-out drops MCP"));
        assert!(SIGN_ON_BOUND.contains("Pro locks until Stiki again"));
        assert!(SIGN_ON_BOUND.contains("Pro unlock = StoreKit + Stiki session"));
        assert!(SIGN_ON_BOUND.contains("Pro surfaces require Stiki session + StoreKit"));
        assert!(SIGN_ON_BOUND.contains("Dictionary, Polish, Clipboard Pro unlimited"));
        assert!(SIGN_ON_BOUND.contains("Insights, Connectors/team, Teams"));
        assert!(SIGN_ON_BOUND.contains("sign-out locks Pro surfaces immediately"));
        assert!(SIGN_ON_BOUND.contains("does NOT promote Scratchpad/dictation/history/Polish/clipboard to Nexus"));
        assert!(SIGN_ON_BOUND.contains("no silent Stiki bootstrap"));
        assert!(SIGN_ON_BOUND.contains("no always-on cookies auto-reconnect MCP without explicit connect"));
        assert!(SIGN_ON_BOUND.contains("no HIPAA/BAA claim"));
        // Canonical BREAKS IF tip.
        assert!(SIGN_ON_BOUND.contains("second IdP / Mabel-native auth parallel to Stiki"));
        assert!(SIGN_ON_BOUND.contains("silent Stiki session enables MCP"));
        assert!(SIGN_ON_BOUND.contains("always-on cookies auto-reconnect MCP without explicit connect"));
        assert!(SIGN_ON_BOUND.contains("Pro unlocks without Stiki"));
        assert!(SIGN_ON_BOUND.contains("Free dictate requires account"));
        assert!(SIGN_ON_BOUND.contains("StoreKit==Stiki collapsed"));
        assert!(SIGN_ON_BOUND.contains("login promotes local→Nexus"));
        assert!(SIGN_ON_BOUND.contains("SSO alone enables MCP"));
        assert!(SIGN_ON_BOUND.contains("Free dictation gated on Stiki"));
        assert!(SIGN_ON_BOUND.contains("Pro surface usable without Stiki"));
        assert!(ERICK_OVERRIDE.contains("Stiki required for ALL Pro unlocks"));
        assert!(ERICK_OVERRIDE.contains("StoreKit ≠ Stiki"));
        assert!(ERICK_OVERRIDE.contains("Pro unlock = StoreKit + Stiki session"));
        assert!(ERICK_OVERRIDE.contains("Dictionary, Polish, Clipboard Pro unlimited"));
        assert!(ERICK_OVERRIDE.contains("Insights, Connectors/team, Teams"));
        assert!(ERICK_OVERRIDE.contains("Free dictation no account"));
        assert!(ERICK_OVERRIDE.contains("Clipboard Free 25 no Stiki"));
        assert!(ERICK_OVERRIDE.contains("sign-out locks Pro immediately"));
        assert!(SIGN_ON_BOUND.contains("sign-out leaves MCP live"));
        assert!(SIGN_ON_BOUND.contains("login/session promotes Scratchpad/dictation/history/Polish/clipboard → Nexus"));
        assert!(SIGN_ON_BOUND.contains("cross-market session silently turns MCP on without explicit connect+ACL"));

        let html = include_str!("../../index.html");
        let ts = include_str!("../../src/main.ts");
        let main = include_str!("main.rs");
        let stiki_src = include_str!("stiki.rs");

        // BREAKS IF: second IdP / Mabel-native auth parallel to Stiki
        assert_eq!(stiki::AUTH_PROVIDERS, &["apple", "google", "microsoft"]);
        assert_eq!(stiki::AUTH_HOST, "auth.chibitek.com");
        assert!(stiki::provider_url("facebook", "http://127.0.0.1/cb").is_err());
        assert!(stiki::provider_url("okta", "http://127.0.0.1/cb").is_err());
        assert!(stiki::provider_url("mabel", "http://127.0.0.1/cb").is_err());
        assert!(!html.contains("id=\"apple-signin\""));
        assert!(!html.contains("id=\"google-signin\""));
        assert!(!html.contains("id=\"microsoft-signin\""));
        assert!(html.contains("id=\"stiki-signin\""));
        assert!(!main.contains("fn apple_sign_in"));
        assert!(!main.contains("fn google_sign_in"));
        let mabel_idp = format!("accounts.{}", "mabel");
        let stiki_prod = stiki_src.split("#[cfg(test)]").next().unwrap();
        assert!(
            !stiki_prod.contains(&mabel_idp),
            "BREAKS IF: second IdP / Mabel-native auth parallel to Stiki"
        );
        assert!(!ts.contains(&mabel_idp));

        // BREAKS IF: silent Stiki session enables MCP
        let dir = tmp();
        inject_session(&dir);
        let signed = status(&dir);
        assert!(signed.signed_in);
        assert!(
            signed.catalog.iter().all(|c| !c.connected && !c.live),
            "BREAKS IF: silent Stiki session enables MCP"
        );
        assert!(
            require_live(&dir, ID_MOCHII).is_err(),
            "BREAKS IF: silent Stiki session enables MCP"
        );
        assert!(require_live(&dir, ID_NEXUS).is_err());
        assert!(
            stiki::require_pro_unlock(&dir).is_err(),
            "BREAKS IF: StoreKit==Stiki collapsed"
        );
        assert!(
            !include_str!("storekit.rs").contains("stiki-session"),
            "BREAKS IF: StoreKit==Stiki collapsed"
        );
        assert!(
            include_str!("teams.rs").contains("require_org_acl"),
            "BREAKS IF: Pro unlocks without Stiki"
        );
        let boot = tmp();
        assert!(
            stiki::sign_in(&boot).is_err(),
            "BREAKS IF: silent Stiki bootstrap"
        );
        assert!(
            !boot.join(stiki::SESSION_FILE).exists(),
            "BREAKS IF: silent Stiki bootstrap"
        );

        // BREAKS IF: always-on cookies auto-reconnect MCP without explicit connect
        let cookies = tmp();
        std::fs::write(cookies.join("cookies"), "stiki=1").unwrap();
        std::fs::write(cookies.join(".stiki-cookies"), "session=1").unwrap();
        assert!(
            stiki::require_session(&cookies).is_err(),
            "BREAKS IF: always-on cookies auto-reconnect MCP without explicit connect"
        );
        assert!(status(&cookies).catalog.iter().all(|c| !c.live));
        assert!(stiki_src.contains("Never cookies"));

        // BREAKS IF: Free dictation gated on Stiki
        assert!(
            !include_str!("recorder.rs").contains("require_session"),
            "BREAKS IF: Free dictate requires account"
        );
        assert!(!include_str!("settings.rs").contains("stiki-session"));
        let dict = ts.split("function renderDictionary").nth(1).unwrap();
        let dict = dict.split("async function persistPolishMode").next().unwrap();
        assert!(!dict.contains("stiki_sign"), "BREAKS IF: Free dictate requires account");
        assert!(
            html.contains("data-view=\"dictionary\" data-pro"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        assert!(
            html.contains("data-view=\"insights\" data-pro"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        assert!(
            html.contains("data-view=\"style\"") && html.contains("data-style-gate"),
            "BREAKS IF: Style usable without Stiki"
        );
        assert!(
            include_str!("style.rs").contains("stiki_session")
                && include_str!("style.rs").contains("ENFORCER_SUITE")
                && include_str!("style.rs").contains("b6530197"),
            "BREAKS IF: Style usable without Stiki"
        );
        assert!(
            html.contains("data-view=\"transforms\"") && html.contains("data-xf-gate"),
            "BREAKS IF: Transforms usable without Stiki"
        );
        assert!(
            include_str!("transforms.rs").contains("stiki_session")
                && include_str!("transforms.rs").contains("ENFORCER_SUITE")
                && include_str!("transforms.rs").contains("b6530197")
                && include_str!("transforms.rs").contains("enforcer_bound_transforms_v1_suite_b6530197"),
            "BREAKS IF: Transforms usable without Stiki"
        );
        assert!(
            html.contains("data-view=\"scratchpad\"") && html.contains("data-scratch-gate"),
            "BREAKS IF: Scratchpad usable without Stiki"
        );
        assert!(
            include_str!("scratchpad.rs").contains("stiki_session")
                && include_str!("scratchpad.rs").contains("ENFORCER_SUITE")
                && include_str!("scratchpad.rs").contains("b6530197")
                && include_str!("scratchpad.rs").contains("enforcer_bound_scratchpad_v1_suite_b6530197"),
            "BREAKS IF: Scratchpad usable without Stiki"
        );
        assert!(
            html.contains("data-view=\"insights\"") && html.contains("data-insights-gate"),
            "BREAKS IF: Insights usable without Stiki"
        );
        assert!(
            include_str!("insights.rs").contains("stiki_session")
                && include_str!("insights.rs").contains("ENFORCER_SUITE")
                && include_str!("insights.rs").contains("b6530197")
                && include_str!("insights.rs").contains("enforcer_bound_insights_v1_suite_b6530197"),
            "BREAKS IF: Insights usable without Stiki"
        );

        // BREAKS IF: Pro surface usable without Stiki
        assert!(
            include_str!("polish.rs").contains("require_pro_unlock"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        assert!(
            include_str!("scratchpad.rs").contains("require_surface"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        assert!(
            include_str!("insights.rs").contains("require_surface"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        assert!(
            include_str!("clipboard_history.rs").contains("pro_unlocked"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        let save = main.split("fn save_settings").nth(1).unwrap();
        let save = save.split("fn polish_set").next().unwrap();
        assert!(
            save.contains("dictionary::surface_ready"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        assert!(
            include_str!("dictionary.rs").contains("stiki_session")
                && include_str!("transcribe_native.rs").contains("effective_terms"),
            "BREAKS IF: Pro surface usable without Stiki"
        );
        assert!(ts.contains("function proUnlocked"));
        assert!(ts.contains("\"dictionary\""));
        assert!(ts.contains("\"insights\""));

        // BREAKS IF: sign-out leaves MCP live
        let mut store = ConnectorStore::default();
        store.mochii.connected = true;
        store.nexus.connected = true;
        write_store(&dir, &store).unwrap();
        let after = sign_out(&dir).unwrap();
        assert!(!after.signed_in);
        assert!(
            after.catalog.iter().all(|c| !c.connected && !c.live),
            "BREAKS IF: sign-out leaves MCP live"
        );
        assert!(stiki::require_session(&dir).is_err());

        // BREAKS IF: login/session promotes Scratchpad/dictation/history/Polish/clipboard → Nexus
        assert!(auto_push_forbidden("dictation").is_err());
        assert!(auto_push_forbidden("history").is_err());
        assert!(auto_push_forbidden("polish").is_err());
        assert!(auto_push_forbidden("clipboard").is_err());
        assert!(scratchpad_as_mcp_source("notes").is_err());
        assert!(scratchpad_as_mcp_sink("notes").is_err());
        let persist = stiki_src
            .split("pub fn persist_valid_identity")
            .nth(1)
            .unwrap();
        let persist = persist.split("pub async fn verify_bearer").next().unwrap();
        assert!(!persist.contains("captures_write"));
        assert!(!persist.contains("connectors::"));

        // BREAKS IF: cross-market session silently turns MCP on without explicit connect+ACL
        let market = tmp();
        let mut leftover = ConnectorStore::default();
        leftover.mochii.connected = true;
        leftover.nexus.connected = true;
        write_store(&market, &leftover).unwrap();
        let cross = accept_identity(
            &market,
            &stiki::Identity {
                subject: "ada@chibitek.com".into(),
                scopes: vec!["mochii".into(), "nexus".into()],
                denied: false,
            },
        )
        .unwrap();
        assert!(cross.signed_in);
        assert!(
            cross.catalog.iter().all(|c| !c.connected && !c.live),
            "BREAKS IF: cross-market session silently turns MCP on without explicit connect+ACL"
        );
        assert!(require_live(&market, ID_MOCHII).is_err());
        assert!(main.contains("accept_identity"));
        assert!(!html.to_ascii_lowercase().contains("hipaa"));
        assert!(html.contains("same as Get Mochii"));
        assert!(html.contains("Sign in with Stiki"));
        assert!(
            html.contains("Free dictation works without an account"),
            "BREAKS IF: UI does not say Free dictation works"
        );
        assert!(
            html.contains("Activate Pro") && html.contains("Sign in with Stiki"),
            "BREAKS IF: Pro features do not ask Sign in with Stiki + Activate Pro"
        );
        assert!(ts.contains("function askProUnlock"));
        assert!(ts.contains("openPlans()"));
        assert!(ts.contains("openAccount"));
    }

    #[test]
    fn cross_market_stiki_session_is_identity_only() {
        assert!(SIGN_ON_ADDENDUM.contains("identity only"));
        assert!(SIGN_ON_ADDENDUM.contains("explicit Connect + ACL"));
        assert!(SIGN_ON_ADDENDUM.contains("SSO alone is never connected"));

        let dir = tmp();
        let mut leftover = ConnectorStore::default();
        leftover.mochii.connected = true;
        leftover.nexus.connected = true;
        write_store(&dir, &leftover).unwrap();

        let s = accept_identity(
            &dir,
            &stiki::Identity {
                subject: "ada@chibitek.com".into(),
                scopes: vec!["mochii".into(), "nexus".into()],
                denied: false,
            },
        )
        .unwrap();
        assert!(s.signed_in, "Get Mochii Stiki session is identity");
        assert!(
            s.catalog.iter().all(|c| !c.connected && !c.live),
            "BREAKS IF: SSO treated as connected"
        );
        assert!(
            require_live(&dir, ID_MOCHII).is_err(),
            "BREAKS IF: SSO treated as connected"
        );
        assert!(require_live(&dir, ID_NEXUS).is_err());
        assert!(
            stiki::issue_user_grant(&dir, ID_MOCHII).is_ok(),
            "ACL is available after identity"
        );
        assert!(
            require_live(&dir, ID_MOCHII).is_err(),
            "BREAKS IF: grant without explicit Connect is live"
        );
        assert!(
            connect(&dir, ID_MOCHII).is_err(),
            "Connect still needs StoreKit Pro after SSO"
        );

        let main = include_str!("main.rs");
        let sign_in = main.split("async fn stiki_sign_in").nth(1).unwrap();
        let sign_in = sign_in.split("fn stiki_sign_out").next().unwrap();
        assert!(
            sign_in.contains("accept_identity"),
            "BREAKS IF: SSO treated as connected"
        );
        assert!(
            !sign_in.contains("connectors::connect("),
            "BREAKS IF: SSO treated as connected"
        );

        let html = include_str!("../../index.html");
        assert!(html.contains("identity only") || html.contains("Identity only"));
        assert!(html.contains("same as Get Mochii"));
        let hint = s.catalog.iter().find(|c| c.id == ID_MOCHII).unwrap();
        assert!(hint.hint.contains("identity only") || hint.hint.contains("SSO is not"));
    }

    #[test]
    fn settings_and_help_name_connectors_without_web_upgrade() {
        let html = include_str!("../../index.html");
        assert!(html.contains("Connectors"));
        assert!(html.contains("Mochii MCP"));
        assert!(html.contains("Nexus MCP"));
        assert!(html.contains("Activate Pro"));
        assert!(html.contains("Plans"));
        assert!(html.contains("not a marketing site") || html.contains("never opens a marketing site"));
        let src = include_str!("connectors.rs");
        assert!(src.contains("Cat UI") || src.contains("cat door") || html.contains("Mabel will not"));
        assert!(html.contains("Mabel will not auto-push") || html.contains("will not auto-push"));
    }

    #[test]
    fn isolated_from_scratchpad_source_and_spatial() {
        let src = include_str!("connectors.rs");
        assert!(src.contains("Scratchpad NOT MCP"));
        assert!(src.contains("export_scratchpad_to_captures"));
        assert!(src.contains("auto_push_forbidden"));
        let pad = include_str!("pro_features.rs");
        assert!(pad.contains("scratchpad.txt"));
        assert!(!pad.contains("connectors.json"));
        assert!(!pad.contains("captures_write"));
        let spatial = include_str!("../../MabelSpatial/MabelSpatial/MabelSpatialApp.swift");
        assert!(!spatial.contains("connectors"));
        assert!(!spatial.contains("Mochii MCP"));
    }
}
