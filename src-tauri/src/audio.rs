use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::dictation_error::{self, UserError};

/// RMS of digital silence / TCC-denied HAL buffers. Quiet speech is orders
/// of magnitude louder after a real mic callback. Stay well below that so we
/// only fail closed on empty MAS/TF captures, not whispered dictation.
pub const DIGITAL_SILENCE_RMS: f32 = 1e-5;

#[derive(Debug)]
pub enum SaveError {
    EmptyBuffer,
    DigitalSilence { rms: f32 },
    Other(String),
}

impl SaveError {
    pub fn to_user_error(&self, elapsed_ms: u64) -> UserError {
        match self {
            SaveError::EmptyBuffer => dictation_error::capture_empty(elapsed_ms),
            SaveError::DigitalSilence { rms } => dictation_error::capture_silent(elapsed_ms, *rms),
            SaveError::Other(s) => {
                UserError::new(dictation_error::TITLE_CAPTURE, format!("Failed to capture audio: {s}"))
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MicDevice {
    pub name: String,
    pub is_default: bool,
}

pub fn list_microphones() -> Vec<MicDevice> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();

    let mut devices = Vec::new();
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                devices.push(MicDevice {
                    is_default: name == default_name,
                    name,
                });
            }
        }
    }
    devices
}

/// Wrapper to make cpal::Stream usable across threads.
/// SAFETY: cpal::Stream on macOS (CoreAudio) is thread-safe in practice;
/// we only access it behind a Mutex to start/stop recording.
struct SendStream(#[allow(dead_code)] cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    stream: Option<SendStream>,
    source_sample_rate: u32,
    source_channels: u16,
    level: Arc<AtomicU32>,
    running: Arc<AtomicBool>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            source_sample_rate: 48000,
            source_channels: 1,
            level: Arc::new(AtomicU32::new(0)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&mut self, app: &AppHandle, mic_name: &str) -> Result<(), String> {
        // Clear any leftover samples from previous recording
        self.samples.lock().unwrap().clear();

        let host = cpal::default_host();

        let device = if mic_name == "default" {
            host.default_input_device()
                .ok_or("No default input device found")?
        } else {
            host.input_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().map(|n| n == mic_name).unwrap_or(false))
                .ok_or(format!("Microphone '{}' not found", mic_name))?
        };

        // Use the device's default config instead of forcing 16kHz
        let default_config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let sample_rate = default_config.sample_rate().0;
        let channels = default_config.channels();

        println!("[Mabel] Mic config: {}Hz, {} channels", sample_rate, channels);

        self.source_sample_rate = sample_rate;
        self.source_channels = channels;

        let config = cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let samples = self.samples.clone();
        let level = self.level.clone();
        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut buf = samples.lock().unwrap();
                    buf.extend_from_slice(data);

                    // RMS for the latest chunk drives the waveform.
                    if !data.is_empty() {
                        let sum_sq: f32 = data.iter().map(|s| s * s).sum();
                        let rms = (sum_sq / data.len() as f32).sqrt();
                        level.store(rms.to_bits(), Ordering::Relaxed);
                    }
                },
                |err| {
                    eprintln!("[Mabel] Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(SendStream(stream));

        // Emit audio-level events at ~30 Hz so the overlay waveform stays smooth
        // without flooding the IPC channel.
        self.running.store(true, Ordering::Relaxed);
        let running = self.running.clone();
        let level_handle = self.level.clone();
        let app_handle = app.clone();
        tokio::spawn(async move {
            while running.load(Ordering::Relaxed) {
                let lvl = f32::from_bits(level_handle.load(Ordering::Relaxed));
                let _ = app_handle.emit("audio-level", lvl);
                tokio::time::sleep(Duration::from_millis(33)).await;
            }
            // One last zero so the bars settle.
            let _ = app_handle.emit("audio-level", 0.0_f32);
        });

        println!("[Mabel] Audio recording started");
        Ok(())
    }

    /// Drain everything captured so far into a 16 kHz mono WAV.
    /// Returns Ok(Some(rms)) with the pre-normalization mean RMS of the chunk
    /// if a file was written, Ok(None) if nothing was buffered. The caller can
    /// gate transcription on the RMS to avoid feeding silence to Whisper, which
    /// otherwise hallucinates ("thank you", "okay", etc) on near-silent input.
    /// Does not stop the stream, so it's safe to call mid-recording.
    pub fn drain_to_wav(&mut self, output_path: &PathBuf) -> Result<Option<f32>, String> {
        let raw = std::mem::take(&mut *self.samples.lock().unwrap());
        if raw.is_empty() {
            return Ok(None);
        }

        let mono: Vec<f32> = if self.source_channels > 1 {
            raw.chunks(self.source_channels as usize)
                .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
                .collect()
        } else {
            raw
        };

        let resampled = resample(&mono, self.source_sample_rate, 16000);

        // Pre-normalization RMS. Driver will use this to skip Whisper for chunks
        // that are mostly silence with brief noise blips.
        let mean_rms = if resampled.is_empty() {
            0.0
        } else {
            (resampled.iter().map(|s| s * s).sum::<f32>() / resampled.len() as f32).sqrt()
        };

        let normalized = peak_normalize(&resampled, 0.707);

        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(output_path, spec).map_err(|e| e.to_string())?;
        for &sample in normalized.iter() {
            let amplitude = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(amplitude).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;
        Ok(Some(mean_rms))
    }

    pub fn stop_and_save(&mut self, output_path: &PathBuf) -> Result<(PathBuf, f32), SaveError> {
        self.running.store(false, Ordering::Relaxed);
        self.stream = None; // Drop stops the stream
        println!("[Mabel] Audio recording stopped");

        let saved = self.drain_to_wav(output_path).map_err(SaveError::Other)?;
        let Some(rms) = saved else {
            return Err(SaveError::EmptyBuffer);
        };
        if is_digital_silence(rms) {
            let _ = std::fs::remove_file(output_path);
            return Err(SaveError::DigitalSilence { rms });
        }
        println!("[Mabel] WAV saved to {:?} (rms={:.6})", output_path, rms);
        Ok((output_path.clone(), rms))
    }

    pub fn current_level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    pub fn stop_stream_only(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.stream = None;
    }
}

/// Peak-normalize a buffer so its loudest sample sits at `target` (0.0 to 1.0).
/// Leaves the buffer alone if the peak is below 0.001 (effectively silent) to
/// avoid amplifying pure noise to full scale.
fn peak_normalize(samples: &[f32], target: f32) -> Vec<f32> {
    let peak = samples.iter().fold(0.0_f32, |acc, &s| acc.max(s.abs()));
    if peak < 0.001 {
        return samples.to_vec();
    }
    let gain = target / peak;
    samples.iter().map(|s| (s * gain).clamp(-1.0, 1.0)).collect()
}

/// Simple linear interpolation resampler
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx = src_idx as usize;
        let frac = src_idx - idx as f64;

        let sample = if idx + 1 < samples.len() {
            samples[idx] as f64 * (1.0 - frac) + samples[idx + 1] as f64 * frac
        } else {
            samples[idx.min(samples.len() - 1)] as f64
        };

        output.push(sample as f32);
    }

    output
}

pub fn is_digital_silence(rms: f32) -> bool {
    !rms.is_finite() || rms <= DIGITAL_SILENCE_RMS
}

/// FluidAudio Parakeet 0.15.6 rejects clips shorter than 1 second
/// (`audioSamples.count >= 16_000` or `ASRError.invalidAudioData`). Short
/// hotkey taps used to fail closed as "Model not ready" after the overlapping
/// toggle race stopped capture almost immediately. Pad with trailing digital
/// silence so the model sees the real speech plus a quiet tail. Whisper.cpp
/// is not padded — Silero VAD would drop the tail, and extra silence there
/// re-opens the "Thanks for watching" hallucination.
pub const NATIVE_ASR_MIN_SAMPLES: u32 = 16_000;
pub const NATIVE_ASR_SAMPLE_RATE: u32 = 16_000;

pub fn native_engine_needs_min_duration(local_engine: &str) -> bool {
    crate::local_engine::is_native_coreml(local_engine)
}

/// Append zero samples to a 16 kHz mono PCM16 WAV until it is at least
/// `min_samples` long. Leaves a longer file untouched. RMS is computed
/// before this runs, so padding cannot hide a silent/TCC-denied capture.
pub fn pad_pcm16_mono_wav(path: &PathBuf, min_samples: u32) -> Result<u32, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    if spec.channels != 1
        || spec.sample_rate != NATIVE_ASR_SAMPLE_RATE
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        return Err(format!(
            "refusing to pad unexpected wav spec: {}Hz {}ch {}bit {:?}",
            spec.sample_rate, spec.channels, spec.bits_per_sample, spec.sample_format
        ));
    }
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let have = samples.len() as u32;
    if have >= min_samples {
        return Ok(have);
    }
    let mut padded = samples;
    padded.resize(min_samples as usize, 0);
    let mut writer = WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for sample in padded {
        writer.write_sample(sample).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())?;
    Ok(min_samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_rms_is_digital_silence() {
        assert!(is_digital_silence(0.0));
        assert!(is_digital_silence(1e-8));
        assert!(is_digital_silence(DIGITAL_SILENCE_RMS));
    }

    #[test]
    fn quiet_speech_is_not_digital_silence() {
        // Streaming VAD treats 0.008 as silence-for-endpoint, but that is
        // still real mic energy — not a TCC-denied zero buffer.
        assert!(!is_digital_silence(0.001));
        assert!(!is_digital_silence(0.008));
        assert!(!is_digital_silence(0.02));
    }

    #[test]
    fn save_error_maps_to_user_visible_copy() {
        let empty = SaveError::EmptyBuffer.to_user_error(2_000);
        assert_eq!(empty.title, dictation_error::TITLE_CAPTURE);
        assert!(empty.message.contains("Microphone"));

        let silent = SaveError::DigitalSilence { rms: 0.0 }.to_user_error(2_000);
        assert!(silent.message.contains("silence"));
    }

    fn write_mono_pcm16(path: &PathBuf, samples: &[i16]) {
        let spec = WavSpec {
            channels: 1,
            sample_rate: NATIVE_ASR_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = WavWriter::create(path, spec).unwrap();
        for &sample in samples {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    fn read_mono_pcm16(path: &PathBuf) -> Vec<i16> {
        let mut reader = hound::WavReader::open(path).unwrap();
        reader.samples::<i16>().map(|s| s.unwrap()).collect()
    }

    #[test]
    fn pad_extends_short_native_wav_to_one_second() {
        let dir = std::env::temp_dir().join(format!(
            "mabel-pad-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("short.wav");
        write_mono_pcm16(&path, &[100, -100, 200, -200]);
        let len = pad_pcm16_mono_wav(&path, NATIVE_ASR_MIN_SAMPLES).unwrap();
        assert_eq!(len, NATIVE_ASR_MIN_SAMPLES);
        let samples = read_mono_pcm16(&path);
        assert_eq!(samples.len(), NATIVE_ASR_MIN_SAMPLES as usize);
        assert_eq!(&samples[..4], &[100, -100, 200, -200]);
        assert!(samples[4..].iter().all(|s| *s == 0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pad_leaves_long_enough_wav_alone() {
        let dir = std::env::temp_dir().join(format!(
            "mabel-pad-long-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("long.wav");
        let original: Vec<i16> = (0..NATIVE_ASR_MIN_SAMPLES + 32).map(|i| (i % 17) as i16).collect();
        write_mono_pcm16(&path, &original);
        let len = pad_pcm16_mono_wav(&path, NATIVE_ASR_MIN_SAMPLES).unwrap();
        assert_eq!(len, original.len() as u32);
        assert_eq!(read_mono_pcm16(&path), original);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_coreml_engines_request_padding() {
        assert!(native_engine_needs_min_duration(crate::local_engine::PARAKEET));
        assert!(native_engine_needs_min_duration(crate::local_engine::WHISPERKIT));
        assert!(!native_engine_needs_min_duration(crate::local_engine::WHISPER_CPP));
    }
}
