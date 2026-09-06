//! Local-only dictation stats. Counts and durations, never content.
//!
//! Persisted to `<app_dir>/stats.json`. The Insights UI reads this directly.
//! Nothing in here ever leaves the device. Recording during Free dictation
//! stays local and does not require Stiki. Insights *read* is Pro-gated.

use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyStat {
    #[serde(default)]
    pub dictations: u64,
    #[serde(default)]
    pub words: u64,
    #[serde(default)]
    pub seconds: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    /// yyyy-mm-dd → counts. BTreeMap so the JSON file stays chronologically sorted.
    #[serde(default)]
    pub daily: BTreeMap<String, DailyStat>,
    #[serde(default)]
    pub total_dictations: u64,
    #[serde(default)]
    pub total_words: u64,
    #[serde(default)]
    pub total_seconds: f64,
}

/// Assumed typing speed for the local time-saved estimate. Never a vendor event.
pub const TYPING_WPM: f64 = 40.0;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StatsSummary {
    pub today: u64,
    pub total: u64,
    pub streak: u32,
    pub total_words: u64,
    /// Average words-per-minute across all dictations. 0 when there's no data.
    pub wpm: u32,
    /// Last 30 days, oldest → newest. Each entry is the dictation count.
    pub last30: Vec<u32>,
    /// Minutes not spent typing, assuming [`TYPING_WPM`]. Counts only. Local file.
    pub time_saved_minutes: u64,
}

impl StatsSummary {
    pub fn empty() -> Self {
        Self {
            today: 0,
            total: 0,
            streak: 0,
            total_words: 0,
            wpm: 0,
            last30: vec![0; 30],
            time_saved_minutes: 0,
        }
    }
}

pub fn time_saved_minutes(total_words: u64) -> u64 {
    if total_words == 0 {
        0
    } else {
        (total_words as f64 / TYPING_WPM).round() as u64
    }
}

pub struct StatsStore {
    path: Mutex<PathBuf>,
    inner: Mutex<Stats>,
}

impl StatsStore {
    pub fn load(app_dir: &PathBuf) -> Self {
        Self::load_with_status(app_dir).0
    }

    pub fn load_with_status(app_dir: &PathBuf) -> (Self, Option<String>) {
        let path = app_dir.join("stats.json");
        match crate::storage::read_existing_json::<Stats>(&path, "Insights history") {
            Ok(Some(inner)) => (
                Self {
                    path: Mutex::new(path),
                    inner: Mutex::new(inner),
                },
                None,
            ),
            Ok(None) => (
                Self {
                    path: Mutex::new(path),
                    inner: Mutex::new(Stats::default()),
                },
                None,
            ),
            Err(e) => (
                Self {
                    path: Mutex::new(path),
                    inner: Mutex::new(Stats::default()),
                },
                Some(e),
            ),
        }
    }

    pub fn record(&self, words: u64, seconds: f64) {
        let today = today_str();
        let path = self.path.lock().unwrap().clone();
        let mut s = self.inner.lock().unwrap();
        let day = s.daily.entry(today).or_default();
        day.dictations += 1;
        day.words += words;
        day.seconds += seconds;
        s.total_dictations += 1;
        s.total_words += words;
        s.total_seconds += seconds;
        let _ = save(&path, &s);
    }

    pub fn summary(&self) -> StatsSummary {
        let s = self.inner.lock().unwrap();
        let today_key = today_str();
        let today = s.daily.get(&today_key).map(|d| d.dictations).unwrap_or(0);
        let streak = current_streak(&s);
        let wpm = if s.total_seconds > 0.0 {
            ((s.total_words as f64) / (s.total_seconds / 60.0)).round() as u32
        } else {
            0
        };
        let last30 = last_30_days(&s);
        StatsSummary {
            today,
            total: s.total_dictations,
            streak,
            total_words: s.total_words,
            wpm,
            last30,
            time_saved_minutes: time_saved_minutes(s.total_words),
        }
    }
}

fn save(path: &PathBuf, s: &Stats) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn current_streak(s: &Stats) -> u32 {
    let mut date = Local::now().date_naive();
    let mut streak = 0u32;
    // Allow today to be empty without breaking the streak.
    if !s.daily.contains_key(&date.format("%Y-%m-%d").to_string()) {
        date = date.pred_opt().unwrap_or(date);
    }
    loop {
        let key = date.format("%Y-%m-%d").to_string();
        if s.daily.get(&key).map(|d| d.dictations).unwrap_or(0) > 0 {
            streak += 1;
            match date.pred_opt() {
                Some(prev) => date = prev,
                None => break,
            }
        } else {
            break;
        }
    }
    streak
}

fn last_30_days(s: &Stats) -> Vec<u32> {
    let today = Local::now().date_naive();
    let mut out = Vec::with_capacity(30);
    for i in (0..30).rev() {
        let d = today
            .checked_sub_signed(chrono::Duration::days(i as i64))
            .unwrap_or(today);
        let key = d.format("%Y-%m-%d").to_string();
        let count = s.daily.get(&key).map(|d| d.dictations as u32).unwrap_or(0);
        out.push(count);
    }
    out
}

// Suppress unused-import warnings on non-macOS toolchains; Datelike is used by chrono internals.
#[allow(dead_code)]
fn _silence_datelike(d: NaiveDate) -> u32 {
    d.day()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_stats_file_is_left_untouched() {
        let dir = std::env::temp_dir().join(format!("mabel-stats-corrupt-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stats.json");
        fs::write(&path, "???broken").unwrap();
        let (_store, err) = StatsStore::load_with_status(&dir);
        assert!(err.as_deref().unwrap_or("").contains("left untouched"), "{err:?}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "???broken");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_stats_is_empty_not_an_error() {
        let dir = std::env::temp_dir().join(format!("mabel-stats-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let (store, err) = StatsStore::load_with_status(&dir);
        assert!(err.is_none());
        assert_eq!(store.summary().total, 0);
        assert!(!dir.join("stats.json").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_container_reload_keeps_existing_stats_bytes() {
        let dir = std::env::temp_dir().join(format!("mabel-stats-keep-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stats.json");
        let raw = r#"{"daily":{"2026-09-01":{"dictations":4,"words":20,"seconds":2.0}},"total_dictations":4,"total_words":20,"total_seconds":2.0}"#;
        fs::write(&path, raw).unwrap();
        let (store, err) = StatsStore::load_with_status(&dir);
        assert!(err.is_none());
        assert_eq!(store.summary().total, 4);
        assert_eq!(store.summary().time_saved_minutes, 1);
        assert_eq!(fs::read_to_string(&path).unwrap(), raw);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn time_saved_is_local_typing_estimate() {
        assert_eq!(time_saved_minutes(0), 0);
        assert_eq!(time_saved_minutes(40), 1);
        assert_eq!(time_saved_minutes(120), 3);
        let empty = StatsSummary::empty();
        assert_eq!(empty.total, 0);
        assert_eq!(empty.time_saved_minutes, 0);
        assert_eq!(empty.last30.len(), 30);
    }
}
