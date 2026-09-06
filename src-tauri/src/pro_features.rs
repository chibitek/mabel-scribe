//! Local Pro feature stores (snippets, style, transforms, scratchpad).
//! Mutations fail closed without a live StoreKit entitlement.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::storekit;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snippet {
    pub id: String,
    pub trigger: String,
    pub expansion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StylePrefs {
    #[serde(default = "default_tone")]
    pub tone: String,
    #[serde(default = "default_casing")]
    pub casing: String,
    #[serde(default = "default_punctuation")]
    pub punctuation: String,
}

fn default_tone() -> String { "default".into() }
fn default_casing() -> String { "as-said".into() }
fn default_punctuation() -> String { "auto".into() }

impl Default for StylePrefs {
    fn default() -> Self {
        Self {
            tone: default_tone(),
            casing: default_casing(),
            punctuation: default_punctuation(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformPrefs {
    #[serde(rename = "fillerWords", default = "default_true")]
    pub filler_words: bool,
    #[serde(default = "default_true")]
    pub grammar: bool,
    #[serde(default = "default_true")]
    pub punctuation: bool,
}

fn default_true() -> bool { true }

impl Default for TransformPrefs {
    fn default() -> Self {
        Self {
            filler_words: true,
            grammar: true,
            punctuation: true,
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


pub fn style_get(app_dir: &PathBuf) -> Result<StylePrefs, String> {
    storekit::require_pro()?;
    Ok(read_json(&app_dir.join("style.json")))
}

pub fn style_save(app_dir: &PathBuf, prefs: StylePrefs) -> Result<StylePrefs, String> {
    storekit::require_pro()?;
    let tone = match prefs.tone.as_str() {
        "default" | "formal" | "casual" => prefs.tone,
        _ => "default".into(),
    };
    let casing = match prefs.casing.as_str() {
        "as-said" | "sentence" | "title" => prefs.casing,
        _ => "as-said".into(),
    };
    let punctuation = match prefs.punctuation.as_str() {
        "auto" | "minimal" => prefs.punctuation,
        _ => "auto".into(),
    };
    let clean = StylePrefs { tone, casing, punctuation };
    write_json(app_dir, "style.json", &clean)?;
    Ok(clean)
}

pub fn transforms_get(app_dir: &PathBuf) -> Result<TransformPrefs, String> {
    storekit::require_pro()?;
    Ok(read_json(&app_dir.join("transforms.json")))
}

pub fn transforms_save(app_dir: &PathBuf, prefs: TransformPrefs) -> Result<TransformPrefs, String> {
    storekit::require_pro()?;
    write_json(app_dir, "transforms.json", &prefs)?;
    Ok(prefs)
}

pub fn scratchpad_get(app_dir: &PathBuf) -> Result<String, String> {
    storekit::require_pro()?;
    Ok(fs::read_to_string(app_dir.join("scratchpad.txt")).unwrap_or_default())
}

pub fn scratchpad_save(app_dir: &PathBuf, text: String) -> Result<(), String> {
    storekit::require_pro()?;
    fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
    fs::write(app_dir.join("scratchpad.txt"), text).map_err(|e| e.to_string())
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
        assert!(style_get(&dir).is_err());
        assert!(style_save(&dir, StylePrefs::default()).is_err());
        assert!(transforms_get(&dir).is_err());
        assert!(scratchpad_get(&dir).is_err());
        assert!(scratchpad_save(&dir, "hello".into()).is_err());
    }

    #[test]
    fn local_snippet_store_add_remove() {
        let dir = tmp();
        let items = snippets_add_local(&dir, "sig".into(), "Best regards".into()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].trigger, "sig");
        assert_eq!(snippets_read(&dir).len(), 1);
        assert!(snippets_add_local(&dir, "SIG".into(), "Dup".into()).is_err());
        let left = snippets_remove_local(&dir, items[0].id.clone()).unwrap();
        assert!(left.is_empty());
    }
}
