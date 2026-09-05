use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

use crate::audio::AudioRecorder;
use crate::cleanup::cleanup_text;
use crate::llm::LlmServer;
use crate::paste::{extract_press_enter_command, paste_text, press_return};
use crate::settings::Settings;
use crate::stats::StatsStore;
use crate::streaming::{self, StreamingHandle};
use crate::system_ui;
use crate::transcribe_groq;
use crate::transcribe_native;

pub const TEMP_RECORDING_WAV: &str = "temp_recording.wav";
pub const LEGACY_LAST_RECORDING_WAV: &str = "last_recording.wav";

/// Delete leftover dictation audio in App Support. Called after a successful
/// transcribe and on launch so a leftover `last_recording.wav` from older
/// builds cannot linger.
pub fn wipe_audio_artifacts(app_dir: &PathBuf) {
    for name in [TEMP_RECORDING_WAV, LEGACY_LAST_RECORDING_WAV] {
        let path = app_dir.join(name);
        if path.exists() {
            match std::fs::remove_file(&path) {
                Ok(()) => println!("[Mabel] wiped leftover audio {:?}", path),
                Err(e) => eprintln!("[Mabel] failed to wipe {:?}: {}", path, e),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum RecordingState {
    Ready,
    Recording,
    Transcribing,
}

fn update_overlay(app: &AppHandle, state: &RecordingState) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        let class = match state {
            RecordingState::Ready => "mic",
            RecordingState::Recording => "mic recording",
            RecordingState::Transcribing => "mic transcribing",
        };
        let js = format!("document.getElementById('mic').className = '{}';", class);
        let _ = overlay.eval(&js);
    }
}

pub struct Recorder {
    state: Arc<Mutex<RecordingState>>,
    audio_recorder: Arc<Mutex<AudioRecorder>>,
    streaming_handle: Arc<Mutex<Option<StreamingHandle>>>,
    streaming_words: Arc<AtomicU64>,
    started_at: Arc<Mutex<Option<Instant>>>,
    stats: Arc<StatsStore>,
    llm_server: Arc<LlmServer>,
}

impl Recorder {
    pub fn new(stats: Arc<StatsStore>, llm_server: Arc<LlmServer>) -> Self {
        Self {
            state: Arc::new(Mutex::new(RecordingState::Ready)),
            audio_recorder: Arc::new(Mutex::new(AudioRecorder::new())),
            streaming_handle: Arc::new(Mutex::new(None)),
            streaming_words: Arc::new(AtomicU64::new(0)),
            started_at: Arc::new(Mutex::new(None)),
            stats,
            llm_server,
        }
    }

    pub fn get_state(&self) -> RecordingState {
        self.state.lock().unwrap().clone()
    }

    pub fn start_recording(
        &self,
        app: &AppHandle,
        mic_name: &str,
        settings: &Settings,
        app_dir: &PathBuf,
    ) -> Result<(), String> {
        {
            let state = self.state.lock().unwrap();
            if *state != RecordingState::Ready {
                return Err("Already recording or transcribing".to_string());
            }
        }

        {
            let mut recorder = self.audio_recorder.lock().unwrap();
            recorder.start(app, mic_name)?;
        }

        self.streaming_words.store(0, Ordering::Relaxed);
        *self.started_at.lock().unwrap() = Some(Instant::now());

        // Temporary safety switch: disable streaming worker until we resolve
        // a shutdown hang seen in stop_and_transcribe on some machines.
        // This keeps dictation reliable by always using full-utterance
        // transcription on stop.
        if settings.streaming && settings.cleanup_mode != "llm" {
            crate::debug_log::append(app_dir, "streaming requested but temporarily disabled");
            *self.streaming_handle.lock().unwrap() = None;
        }

        *self.state.lock().unwrap() = RecordingState::Recording;
        let _ = app.emit("recording-state", RecordingState::Recording);
        update_overlay(app, &RecordingState::Recording);

        // Intentionally NOT playing a start sound here. Spawning afplay right
        // after the cpal input stream opens triggers an audio session
        // reconfiguration on Apple Silicon and the mic gain drops to near-zero,
        // which makes Whisper see only ambient noise. Bug-fixed post v1.0.0.

        Ok(())
    }

    pub async fn stop_and_transcribe(
        &self,
        app: &AppHandle,
        settings: &Settings,
        app_dir: &PathBuf,
    ) -> Result<String, String> {
        crate::debug_log::append(app_dir, "stop_and_transcribe entered");
        crate::debug_log::append(
            app_dir,
            &format!(
                "settings snapshot: engine={} streaming={} cleanup_mode={}",
                settings.engine, settings.streaming, settings.cleanup_mode
            ),
        );
        println!("[Mabel] stop_and_transcribe entered");
        {
            crate::debug_log::append(app_dir, "transitioning recorder state to Transcribing");
            let mut state = self.state.lock().unwrap();
            if *state != RecordingState::Recording {
                crate::debug_log::append(
                    app_dir,
                    &format!("stop ignored: recorder state was {:?}", *state),
                );
                return Err("Not currently recording".to_string());
            }
            *state = RecordingState::Transcribing;
            let _ = app.emit("recording-state", RecordingState::Transcribing);
            update_overlay(app, &RecordingState::Transcribing);
        }
        crate::debug_log::append(app_dir, "recorder state is Transcribing");

        let elapsed_seconds = self
            .started_at
            .lock()
            .unwrap()
            .take()
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);

        let streaming_handle = self.streaming_handle.lock().unwrap().take();
        if let Some(handle) = streaming_handle {
            handle.stop().await;
            {
                let mut recorder = self.audio_recorder.lock().unwrap();
                recorder.stop_stream_only();
            }
            streaming::flush_final_chunk(
                app,
                self.audio_recorder.clone(),
                settings,
                app_dir,
                self.streaming_words.clone(),
            )
            .await;

            let words = self.streaming_words.load(Ordering::Relaxed);
            if words > 0 {
                self.stats.record(words, elapsed_seconds);
            }

            *self.state.lock().unwrap() = RecordingState::Ready;
            let _ = app.emit("recording-state", RecordingState::Ready);
            let _ = app.emit("stats-updated", ());
            update_overlay(app, &RecordingState::Ready);
            if settings.dictation_sounds {
                system_ui::play_sound("Pop");
            }
            return Ok(String::new());
        }

        let temp_path = app_dir.join(TEMP_RECORDING_WAV);
        let stop_and_save_result = {
            let mut recorder = self.audio_recorder.lock().unwrap();
            recorder.stop_and_save(&temp_path)
        };
        if let Err(err) = stop_and_save_result {
            crate::debug_log::append(app_dir, &format!("stop_and_save failed: {}", err));
            eprintln!("[Mabel] stop_and_save failed: {}", err);

            {
                let mut state = self.state.lock().unwrap();
                *state = RecordingState::Ready;
                let _ = app.emit("recording-state", RecordingState::Ready);
                let _ = app.emit("stats-updated", ());
                update_overlay(app, &RecordingState::Ready);
            }

            let msg = format!("Failed to capture audio: {}", err);
            let _ = app.emit("transcription-error", msg.clone());
            return Err(msg);
        }
        if let Ok(meta) = std::fs::metadata(&temp_path) {
            crate::debug_log::append(app_dir, &format!("captured temp wav {} bytes", meta.len()));
            println!("[Mabel] Captured temp WAV: {} bytes", meta.len());
        } else {
            crate::debug_log::append(app_dir, "captured temp wav metadata unavailable");
            println!("[Mabel] Captured temp WAV: metadata unavailable");
        }

        let result: Result<String, String> = async {
            crate::debug_log::append(
                app_dir,
                &format!("transcription engine={}", settings.engine),
            );
            println!("[Mabel] Transcription engine: {}", settings.engine);
            let raw_text = match settings.engine.as_str() {
                "local" => {
                    crate::debug_log::append(
                        app_dir,
                        &format!("local transcription start engine={}", settings.local_engine),
                    );
                    println!(
                        "[Mabel] Local transcription starting ({})",
                        settings.local_engine
                    );
                    transcribe_native::transcribe_local_engine(app, app_dir, &temp_path, &settings)
                        .await?
                }
                "cloud" => {
                    crate::debug_log::append(app_dir, "cloud transcription start");
                    println!("[Mabel] Cloud transcription starting");
                    let key = crate::secrets::get_groq_key()?;
                    transcribe_groq::transcribe_groq(&key, &temp_path, &settings.whisper_language)
                        .await?
                }
                _ => return Err(format!("Unknown engine: {}", settings.engine)),
            };
            println!(
                "[Mabel] Transcription returned (chars={})",
                raw_text.chars().count()
            );
            crate::debug_log::append(
                app_dir,
                &format!("transcription returned chars={}", raw_text.chars().count()),
            );

            let rule_cleaned = cleanup_text(&raw_text);
            let cleaned = if settings.cleanup_mode == "llm" && !rule_cleaned.is_empty() {
                println!(
                    "[Mabel] LLM cleanup requested (chars={})",
                    rule_cleaned.chars().count()
                );
                let t0 = std::time::Instant::now();
                let llm_result = match crate::llm::model_filename(&settings.llm_model) {
                    Ok(name) => {
                        let model_path = app_dir.join(name);
                        crate::llm::ensure_and_cleanup(
                            app,
                            &self.llm_server,
                            &settings.llm_model,
                            &model_path,
                            &rule_cleaned,
                        )
                        .await
                    }
                    Err(e) => Err(e),
                };
                match llm_result {
                    Ok(s) if !s.is_empty() => {
                        println!(
                            "[Mabel] LLM cleanup succeeded ({:?}, chars={})",
                            t0.elapsed(),
                            s.chars().count()
                        );
                        s
                    }
                    Ok(empty) => {
                        println!(
                            "[Mabel] LLM returned empty ({:?}, chars={}); falling back to rules",
                            t0.elapsed(),
                            empty.chars().count()
                        );
                        rule_cleaned
                    }
                    Err(e) => {
                        eprintln!(
                            "[Mabel] LLM cleanup failed ({:?}), using rules: {}",
                            t0.elapsed(),
                            e
                        );
                        rule_cleaned
                    }
                }
            } else {
                rule_cleaned
            };

            let (to_paste, press_enter) =
                extract_press_enter_command(&cleaned, settings.press_enter_command);
            println!(
                "[Mabel] Final text prepared (chars={}, press_enter={})",
                to_paste.chars().count(),
                press_enter
            );
            crate::debug_log::append(
                app_dir,
                &format!(
                    "final text chars={} press_enter={}",
                    to_paste.chars().count(),
                    press_enter
                ),
            );
            if !to_paste.is_empty() {
                crate::debug_log::append(app_dir, "pasting text");
                println!("[Mabel] Pasting text");
                paste_text(&to_paste)?;
                crate::debug_log::append(app_dir, "paste command completed");
                let words = to_paste.split_whitespace().count() as u64;
                if words > 0 {
                    self.stats.record(words, elapsed_seconds);
                }
            }
            if press_enter {
                std::thread::sleep(std::time::Duration::from_millis(50));
                let _ = press_return();
            }

            Ok(to_paste)
        }
        .await;

        let _ = std::fs::remove_file(&temp_path);

        {
            let mut state = self.state.lock().unwrap();
            *state = RecordingState::Ready;
            let _ = app.emit("recording-state", RecordingState::Ready);
            let _ = app.emit("stats-updated", ());
            update_overlay(app, &RecordingState::Ready);
        }

        match result {
            Ok(text) => {
                crate::debug_log::append(app_dir, "stop_and_transcribe succeeded");
                println!("[Mabel] stop_and_transcribe succeeded");
                wipe_audio_artifacts(app_dir);
                if settings.dictation_sounds {
                    system_ui::play_sound("Pop");
                }
                Ok(text)
            }
            Err(err) => {
                crate::debug_log::append(
                    app_dir,
                    &format!("transcription pipeline failed: {}", err),
                );
                eprintln!("[Mabel] Transcription pipeline failed: {}", err);
                let _ = app.emit("transcription-error", err.clone());
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_ready() {
        let stats = Arc::new(StatsStore::load(&PathBuf::from("/tmp/mabel-test-recorder")));
        let recorder = Recorder::new(stats, Arc::new(LlmServer::new()));
        assert_eq!(recorder.get_state(), RecordingState::Ready);
    }

    #[test]
    fn wipe_audio_artifacts_deletes_temp_and_legacy_wav() {
        let dir = std::env::temp_dir().join(format!(
            "mabel-wav-wipe-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let temp = dir.join(TEMP_RECORDING_WAV);
        let legacy = dir.join(LEGACY_LAST_RECORDING_WAV);
        std::fs::write(&temp, b"temp").unwrap();
        std::fs::write(&legacy, b"legacy").unwrap();
        wipe_audio_artifacts(&dir);
        assert!(!temp.exists());
        assert!(!legacy.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
