//! Local Pro feature stores (snippets, style, transforms, scratchpad).
//! Mutations fail closed without a live StoreKit entitlement.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snippet {
    pub id: String,
    pub trigger: String,
    pub expansion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StylePrefs {
    /// Formal | Casual | Very casual register. Default OFF/unset until picked.
    /// Not Polish tone. Not freeform casing/punctuation prefs.
    #[serde(default = "default_style_mode")]
    pub mode: String,
}

fn default_style_mode() -> String { "off".into() }

impl Default for StylePrefs {
    fn default() -> Self {
        Self {
            mode: default_style_mode(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformPrefs {
    /// Last user-invoked action: email | bullets | shorter | clearer.
    /// Empty until the user invokes. Not Style register. Not Polish tone.
    /// Not auto-on filler/grammar/punctuation prefs.
    #[serde(rename = "lastAction", default)]
    pub last_action: String,
    #[serde(rename = "lastSource", default)]
    pub last_source: String,
    #[serde(rename = "lastResult", default)]
    pub last_result: String,
}

impl Default for TransformPrefs {
    fn default() -> Self {
        Self {
            last_action: String::new(),
            last_source: String::new(),
            last_result: String::new(),
        }
    }
}

fn read_json<T: for<'de> Deserialize<'de> + Default>(path: &PathBuf) -> T {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_json<T: Serialize>(app_dir: &PathBuf, name: &str, value: &T) -> Result<(), String> {
    fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(app_dir.join(name), json).map_err(|e| e.to_string())
}

fn next_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{n:x}")
}

/// Ungated local read. Product Snippets applies the dual gate before expand / mutate.
pub fn snippets_read(app_dir: &PathBuf) -> Vec<Snippet> {
    read_json(&app_dir.join("snippets.json"))
}

pub fn snippets_add_local(
    app_dir: &PathBuf,
    trigger: String,
    expansion: String,
) -> Result<Vec<Snippet>, String> {
    let trigger = trigger.trim().to_string();
    let expansion = expansion.trim().to_string();
    if trigger.is_empty() || expansion.is_empty() {
        return Err("Snippet needs a trigger and expansion".into());
    }
    let mut items: Vec<Snippet> = snippets_read(app_dir);
    if items.iter().any(|s| s.trigger.eq_ignore_ascii_case(&trigger)) {
        return Err("That trigger already exists".into());
    }
    items.push(Snippet {
        id: next_id(),
        trigger,
        expansion,
    });
    write_json(app_dir, "snippets.json", &items)?;
    Ok(items)
}

pub fn snippets_update_local(
    app_dir: &PathBuf,
    snippet_id: String,
    trigger: String,
    expansion: String,
) -> Result<Vec<Snippet>, String> {
    let trigger = trigger.trim().to_string();
    let expansion = expansion.trim().to_string();
    if trigger.is_empty() || expansion.is_empty() {
        return Err("Snippet needs a trigger and expansion".into());
    }
    let mut items: Vec<Snippet> = snippets_read(app_dir);
    let idx = items
        .iter()
        .position(|s| s.id == snippet_id)
        .ok_or_else(|| "Snippet not found".to_string())?;
    if items
        .iter()
        .enumerate()
        .any(|(i, s)| i != idx && s.trigger.eq_ignore_ascii_case(&trigger))
    {
        return Err("That trigger already exists".into());
    }
    items[idx].trigger = trigger;
    items[idx].expansion = expansion;
    write_json(app_dir, "snippets.json", &items)?;
    Ok(items)
}

pub fn snippets_remove_local(app_dir: &PathBuf, snippet_id: String) -> Result<Vec<Snippet>, String> {
    let mut items: Vec<Snippet> = snippets_read(app_dir);
    let before = items.len();
    items.retain(|s| s.id != snippet_id);
    if items.len() == before {
        return Err("Snippet not found".into());
    }
    write_json(app_dir, "snippets.json", &items)?;
    Ok(items)
}


/// Ungated local read. Product Style applies the dual gate before apply / mutate.
pub fn style_read(app_dir: &PathBuf) -> StylePrefs {
    read_json(&app_dir.join("style.json"))
}

pub fn style_write_local(app_dir: &PathBuf, prefs: StylePrefs) -> Result<StylePrefs, String> {
    let mode = match prefs.mode.trim().to_ascii_lowercase().as_str() {
        "formal" => "formal".into(),
        "casual" => "casual".into(),
        "very-casual" | "very_casual" | "very casual" => "very-casual".into(),
        _ => "off".into(),
    };
    let clean = StylePrefs { mode };
    write_json(app_dir, "style.json", &clean)?;
    Ok(clean)
}

/// Ungated local read. Product Transforms applies the dual gate before rewrite / mutate.
pub fn transforms_read(app_dir: &PathBuf) -> TransformPrefs {
    read_json(&app_dir.join("transforms.json"))
}

pub fn transforms_write_local(
    app_dir: &PathBuf,
    prefs: TransformPrefs,
) -> Result<TransformPrefs, String> {
    let action = match prefs.last_action.trim().to_ascii_lowercase().as_str() {
        "email" | "e-mail" => "email".into(),
        "bullets" | "bullet" | "bullet-points" | "bullet points" => "bullets".into(),
        "shorter" | "make-shorter" | "make shorter" | "shorten" => "shorter".into(),
        "clearer" | "make-clearer" | "make clearer" | "clarify" => "clearer".into(),
        _ => String::new(),
    };
    let clean = TransformPrefs {
        last_action: action,
        last_source: prefs.last_source,
        last_result: prefs.last_result,
    };
    write_json(app_dir, "transforms.json", &clean)?;
    Ok(clean)
}

/// Ungated local read. Product Scratchpad applies the dual gate before show / mutate.
pub fn scratchpad_read(app_dir: &PathBuf) -> String {
    fs::read_to_string(app_dir.join("scratchpad.txt")).unwrap_or_default()
}

pub fn scratchpad_write_local(app_dir: &PathBuf, text: String) -> Result<String, String> {
    fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
    fs::write(app_dir.join("scratchpad.txt"), &text).map_err(|e| e.to_string())?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn tmp() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "mabel-pro-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn pro_stores_fail_closed_without_entitlement() {
        let dir = tmp();
        assert!(crate::snippets::require_list(&dir).is_err());
        assert!(crate::snippets::add(&dir, "sig".into(), "Best".into()).is_err());
        assert!(crate::style::require_prefs(&dir).is_err());
        assert!(crate::style::set_mode(&dir, "formal".into()).is_err());
        assert!(crate::transforms::require_prefs(&dir).is_err());
        assert!(crate::transforms::apply_local(&dir, "email".into(), "hi".into()).is_err());
        assert!(crate::scratchpad::require_text(&dir).is_err());
        assert!(crate::scratchpad::save(&dir, "hello".into()).is_err());
        let stats = crate::stats::StatsStore::load(&dir);
        assert!(crate::insights::require_summary(&stats).is_err());
    }

    #[test]
    fn local_scratchpad_store_persists_notes() {
        let dir = tmp();
        let written = scratchpad_write_local(&dir, "ship friday".into()).unwrap();
        assert_eq!(written, "ship friday");
        assert_eq!(scratchpad_read(&dir), "ship friday");
        let cleared = scratchpad_write_local(&dir, String::new()).unwrap();
        assert!(cleared.is_empty());
        assert!(scratchpad_read(&dir).is_empty());
    }

    #[test]
    fn local_snippet_store_add_remove() {
        let dir = tmp();
        let items = snippets_add_local(&dir, "sig".into(), "Best regards".into()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].trigger, "sig");
        assert_eq!(snippets_read(&dir).len(), 1);
        assert!(snippets_add_local(&dir, "SIG".into(), "Dup".into()).is_err());
        let edited = snippets_update_local(
            &dir,
            items[0].id.clone(),
            "signature".into(),
            "Best regards".into(),
        )
        .unwrap();
        assert_eq!(edited[0].trigger, "signature");
        assert!(snippets_update_local(&dir, "missing".into(), "x".into(), "y".into()).is_err());
        let left = snippets_remove_local(&dir, items[0].id.clone()).unwrap();
        assert!(left.is_empty());
    }

    #[test]
    fn local_transform_store_persists_last_invoke() {
        let dir = tmp();
        let written = transforms_write_local(
            &dir,
            TransformPrefs {
                last_action: "email".into(),
                last_source: "ship friday".into(),
                last_result: "Subject: ship friday\n\nship friday".into(),
            },
        )
        .unwrap();
        assert_eq!(written.last_action, "email");
        let loaded = transforms_read(&dir);
        assert_eq!(loaded.last_action, "email");
        assert_eq!(loaded.last_source, "ship friday");
        assert!(loaded.last_result.contains("ship friday"));
        let cleared = transforms_write_local(&dir, TransformPrefs::default()).unwrap();
        assert!(cleared.last_action.is_empty());
        assert!(transforms_read(&dir).last_result.is_empty());
    }
}
