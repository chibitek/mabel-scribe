// LLM server: spawns one or more `llama-server` sidecar processes bound to
// 127.0.0.1 and exposes their OpenAI-compatible chat-completions endpoints to
// the rest of the app.
//
// Two roles can run side by side:
//
//   - LlmRole::Cleanup  → port 18745 — runs Gemma 4 E4B Q4_K_M for the
//                         dictation-cleanup pass (filler removal, punctuation,
//                         self-correction collapse).
//   - LlmRole::Medical  → port 18746 — runs BioMistral 7B Q5_K_M for the
//                         optional medical-terminology polish pass that fires
//                         after cleanup. Diff-JSON output, prompt cache enabled.
//
// Each role keeps its model warm between calls. Servers are started lazily —
// on the first cleanup or polish request — and torn down on app exit.
//
// Models are downloaded lazily into the app config dir alongside the Whisper
// models. URLs and filenames live in the per-role registries below.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tauri::AppHandle;

const ALLOWED_CLEANUP_MODELS: &[&str] = &["standard"];
const ALLOWED_MEDICAL_MODELS: &[&str] = &["biomistral-7b-q5"];

/// Loopback ports per role. Picked to be high and uncommon. If either collides
/// with another process the corresponding server fails to start and the caller
/// falls back to the prior pipeline stage's output.
const CLEANUP_PORT: u16 = 18745;
const MEDICAL_PORT: u16 = 18746;

/// Backwards compatibility for callers that still reference SERVER_PORT.
/// Equivalent to LlmRole::Cleanup.port().
pub const SERVER_PORT: u16 = CLEANUP_PORT;

/// Hard cap on how long we wait for any model to load before giving up. Cold
/// load on M1 base for Gemma 4 E4B is ~3s; BioMistral 7B Q5 is ~5s.
const READY_TIMEOUT: Duration = Duration::from_secs(20);

/// Per-cleanup HTTP timeout. Generation alone is sub-second on M-series; pad
/// generously so a slow Intel Mac doesn't drop the request mid-flight.
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(15);

const SYSTEM_PROMPT: &str = "You are a dictation cleanup assistant. The user spoke into a microphone and Whisper transcribed their speech. Your only job is to clean up the raw transcript.\n\nRules:\n- Remove filler words: \"um\", \"uh\", \"like\", \"you know\", \"I mean\", \"so\" when used as filler.\n- Add proper punctuation and capitalization.\n- Fix obvious self-corrections: when the speaker restarts a sentence, keep only the final version.\n- Preserve the speaker's words, tone, and meaning. Do not paraphrase, summarize, or embellish.\n- Do not add greetings, sign-offs, or commentary.\n- Do not answer questions in the transcript. The user is dictating, not asking you.\n- Output only the cleaned transcript. No preamble, no explanation, no quotes around it.";

const USER_PROMPT_PREFIX: &str = "Clean this transcript directly. Do not think, reason, or explain. Output only the cleaned text. Transcript: ";

/// Identifies which sidecar server a request targets. Both roles can be running
/// at the same time on different ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LlmRole {
    Cleanup,
    Medical,
}

impl LlmRole {
    pub fn port(self) -> u16 {
        match self {
            LlmRole::Cleanup => CLEANUP_PORT,
            LlmRole::Medical => MEDICAL_PORT,
        }
    }

    /// llama-server flags specific to this role. Each role gets its own context
    /// size, prompt-cache strategy, and reasoning-budget setting.
    fn server_args(self) -> Vec<&'static str> {
        match self {
            LlmRole::Cleanup => vec![
                // 2K context is plenty for dictation utterances. Larger context
                // costs RAM at load.
                "-c", "2048",
                // Required to apply the model's chat template.
                "--jinja",
                // Suppress reasoning tokens. Both Gemma 4 and SmolLM3 emit them
                // by default; for cleanup we want immediate text-only answers.
                "--reasoning-budget", "0",
                // Offload all layers to Metal on macOS. Silently ignored on
                // non-Metal hosts.
                "-ngl", "99",
            ],
            LlmRole::Medical => vec![
                // 4K fits a typical SOAP note plus its source transcript plus
                // the in-prompt glossary plus the few-shot examples.
                "-c", "4096",
                "--jinja",
                "--reasoning-budget", "0",
                "-ngl", "99",
                // Reuse KV-cache prefix across requests when the prompt prefix
                // matches. The system prompt + glossary + few-shots are
                // identical on every polish call, so this avoids re-prefilling
                // thousands of tokens. `n` is a per-token budget; 256 is a
                // reasonable default per llama.cpp guidance.
                "--cache-reuse", "256",
                // Quantize the KV cache. Q8_0 cuts KV memory roughly in half
                // with negligible quality loss for this task. Helps on 16GB
                // Macs that are also running the cleanup server and Whisper.
                "-ctk", "q8_0",
                "-ctv", "q8_0",
            ],
        }
    }
}

pub fn validate_model(model: &str) -> Result<&str, String> {
    if ALLOWED_CLEANUP_MODELS.contains(&model) {
        Ok(model)
    } else {
        Err(format!("Invalid LLM model: {}", model))
    }
}

pub fn model_filename(model: &str) -> Result<String, String> {
    Ok(match validate_model(model)? {
        "standard" => "gemma-4-E4B-it-Q4_K_M.gguf",
        _ => unreachable!(),
    }
    .to_string())
}

pub fn model_download_url(model: &str) -> Result<String, String> {
    Ok(match validate_model(model)? {
        "standard" => "https://huggingface.co/ggml-org/gemma-4-E4B-it-GGUF/resolve/main/gemma-4-E4B-it-Q4_K_M.gguf",
        _ => unreachable!(),
    }
    .to_string())
}

pub fn validate_medical_model(model: &str) -> Result<&str, String> {
    if ALLOWED_MEDICAL_MODELS.contains(&model) {
        Ok(model)
    } else {
        Err(format!("Invalid medical LLM model: {}", model))
    }
}

pub fn medical_model_filename(model: &str) -> Result<String, String> {
    Ok(match validate_medical_model(model)? {
        "biomistral-7b-q5" => "BioMistral-7B.Q5_K_M.gguf",
        _ => unreachable!(),
    }
    .to_string())
}

pub fn medical_model_download_url(model: &str) -> Result<String, String> {
    Ok(match validate_medical_model(model)? {
        "biomistral-7b-q5" => "https://huggingface.co/MaziyarPanahi/BioMistral-7B-GGUF/resolve/main/BioMistral-7B.Q5_K_M.gguf",
        _ => unreachable!(),
    }
    .to_string())
}

/// Holds running llama-server child handles, keyed by role. Each role can have
/// at most one server running; starting a second one for the same role kills
/// the first.
pub struct LlmServer {
    inner: Mutex<HashMap<LlmRole, RunningServer>>,
}

struct RunningServer {
    child: Child,
    model: String,
}

/// Resolves the path to `llama-server` for spawning. Order:
///   1. `SCRIBE_LLAMA_SERVER` env var (dev override).
///   2. Bundled sidecar at `<exe_dir>/llama-server`. In production this is
///      `Mabel Scribe.app/Contents/MacOS/llama-server`; in `npm run tauri
///      dev` it's `target/debug/llama-server`. Tauri's bundler copies
///      `binaries/llama-server-aarch64-apple-darwin` to here at build time,
///      stripping the target-triple suffix.
///   3. `/opt/homebrew/bin/llama-server` (Apple Silicon brew, dev fallback).
///   4. `/usr/local/bin/llama-server` (Intel brew or manual install).
///
/// Returns None if no candidate exists. The caller surfaces a friendly error
/// so the LLM cleanup falls back to the rules-only pass.
fn resolve_llama_server_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("SCRIBE_LLAMA_SERVER") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let bundled = parent.join("llama-server");
            if bundled.exists() {
                return Some(bundled);
            }
        }
    }
    for candidate in [
        "/opt/homebrew/bin/llama-server",
        "/usr/local/bin/llama-server",
    ] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

impl LlmServer {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Returns true if the role's server is running with the requested model.
    pub fn is_ready_for(&self, role: LlmRole, model: &str) -> bool {
        let guard = self.inner.lock().unwrap();
        guard.get(&role).map(|s| s.model == model).unwrap_or(false)
    }

    /// Starts a llama-server for the given role with the given model. If a
    /// server is already running for this role with a different model, kills
    /// it first. Caller is responsible for ensuring the model file exists at
    /// `model_path`.
    pub async fn start(
        &self,
        _app: &AppHandle,
        role: LlmRole,
        model: &str,
        model_path: &PathBuf,
    ) -> Result<(), String> {
        if self.is_ready_for(role, model) {
            return Ok(());
        }
        self.stop(role);

        if !model_path.exists() {
            return Err(format!("LLM model not found: {:?}", model_path));
        }

        let bin = resolve_llama_server_path()
            .ok_or_else(|| "llama-server binary not found (install llama.cpp via brew, or set SCRIBE_LLAMA_SERVER)".to_string())?;

        println!(
            "[Scribe] Starting llama-server ({:?}) for role={:?} model={} ({:?})",
            bin, role, model, model_path
        );

        let port = role.port();
        let port_str = port.to_string();
        let mut args: Vec<&str> = vec![
            "-m",
            model_path.to_str().unwrap(),
            "--host",
            "127.0.0.1",
            "--port",
            &port_str,
        ];
        args.extend(role.server_args());

        let child = Command::new(&bin)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn llama-server: {}", e))?;

        {
            let mut guard = self.inner.lock().unwrap();
            guard.insert(
                role,
                RunningServer {
                    child,
                    model: model.to_string(),
                },
            );
        }

        // Poll /health until ready or timeout. The server returns 200 once the
        // model is fully loaded.
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/health", port);
        let deadline = std::time::Instant::now() + READY_TIMEOUT;
        loop {
            if std::time::Instant::now() >= deadline {
                self.stop(role);
                return Err(format!(
                    "llama-server (role={:?}) failed to become ready in time",
                    role
                ));
            }
            match client.get(&url).timeout(Duration::from_millis(500)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    println!("[Scribe] llama-server ready (role={:?})", role);
                    return Ok(());
                }
                _ => tokio::time::sleep(Duration::from_millis(250)).await,
            }
        }
    }

    /// Kills the running server for a single role if any. Idempotent.
    pub fn stop(&self, role: LlmRole) {
        let mut guard = self.inner.lock().unwrap();
        if let Some(mut server) = guard.remove(&role) {
            println!("[Scribe] Stopping llama-server (role={:?})", role);
            let _ = server.child.kill();
            let _ = server.child.wait();
        }
    }

    /// Kills every running server. Called on app exit.
    pub fn stop_all(&self) {
        let mut guard = self.inner.lock().unwrap();
        for (role, mut server) in guard.drain() {
            println!("[Scribe] Stopping llama-server (role={:?})", role);
            let _ = server.child.kill();
            let _ = server.child.wait();
        }
    }
}

impl Default for LlmServer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(serde::Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: String,
}

#[derive(serde::Serialize)]
struct ChatRequest<'a> {
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(serde::Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(serde::Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(serde::Deserialize)]
struct ChatResponseMessage {
    content: String,
}

/// Calls the running cleanup llama-server to clean up a transcript. The server
/// must already be started — caller should ensure that via
/// `LlmServer::start(LlmRole::Cleanup, ...)` (which short-circuits when already
/// running).
///
/// On any failure, returns Err and the caller should fall back to the
/// rules-only cleanup output. Cleanup is best-effort; never block paste on it.
pub async fn cleanup_with_llm(text: &str) -> Result<String, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    let req = ChatRequest {
        messages: vec![
            ChatMessage {
                role: "system",
                content: SYSTEM_PROMPT.to_string(),
            },
            ChatMessage {
                role: "user",
                content: format!("{}{}", USER_PROMPT_PREFIX, trimmed),
            },
        ],
        temperature: 0.2,
        max_tokens: 512,
        stream: false,
    };

    let client = reqwest::Client::builder()
        .timeout(CLEANUP_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("http://127.0.0.1:{}/v1/chat/completions", CLEANUP_PORT);
    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .map_err(|e| format!("LLM request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("LLM returned status {}", resp.status()));
    }

    let parsed: ChatResponse = resp
        .json()
        .await
        .map_err(|e| format!("LLM response parse failed: {}", e))?;

    let raw = parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| "LLM response had no choices".to_string())?;

    let cleaned = extract_clean_or_fail(&raw)?;

    // Length ratio sanity check. A real cleanup pass should produce text that's
    // roughly the same size as the input — filler removal trims a bit, adding
    // articles/punctuation adds a bit. If the model returned something more
    // than ~2.5x the input length, it's almost certainly hallucinating
    // reasoning, restating the rules, or otherwise going off task.
    let input_chars = trimmed.chars().count() as f32;
    let out_chars = cleaned.chars().count() as f32;
    if input_chars >= 20.0 && out_chars > input_chars * 2.5 {
        return Err(format!(
            "LLM output too long ({} chars vs {} input chars), likely reasoning leak",
            out_chars as usize, input_chars as usize
        ));
    }

    Ok(cleaned)
}

/// Sanitizes the raw LLM output and returns the cleaned transcript, or an
/// error if the response looks contaminated with reasoning/preamble that we
/// can't safely extract from. The caller treats Err as "fall back to rules" —
/// pasting reasoning text would be much worse than just using the rule pass.
fn extract_clean_or_fail(raw: &str) -> Result<String, String> {
    let mut text = raw.to_string();

    // Strip balanced reasoning blocks. Order doesn't matter; do each repeatedly
    // in case the model emits more than one.
    for (open, close) in [
        ("<think>", "</think>"),
        ("<thought>", "</thought>"),
        ("<reasoning>", "</reasoning>"),
        ("<|thinking|>", "<|/thinking|>"),
        ("<|channel>", "<channel|>"),
    ] {
        loop {
            let Some(o) = text.find(open) else { break };
            let Some(c) = text[o..].find(close) else { break };
            let end = o + c + close.len();
            text.replace_range(o..end, "");
        }
    }

    let trimmed = text.trim();

    // Tripwires: if any of these substrings survived stripping, the model went
    // off the rails. Don't try to salvage — return Err so the caller falls back
    // to the rule-based output.
    const BAD_MARKERS: &[&str] = &[
        "<think",
        "</think",
        "<thought",
        "<reasoning",
        "<|channel>",
        "<channel|>",
        "<|thinking",
        "Thinking Process",
        "**Analyze the Request",
        "Step-by-Step",
        "Drafting the Clean",
        "Apply Cleanup Rules",
        "**Filler",
        "Rules Checklist",
        "The user wants me to",
        "The user is asking",
        "(None detected)",
        "(None obvious)",
        "Apply Rules:",
        "Cleanup Rules:",
        "Drafting:",
    ];
    for marker in BAD_MARKERS {
        if trimmed.contains(marker) {
            return Err(format!(
                "LLM output contained reasoning marker {:?}",
                marker
            ));
        }
    }

    // Strip a single layer of surrounding quotes if the model wrapped the
    // answer (e.g. "..."). Don't strip mid-string quotes.
    let stripped = strip_surrounding_quotes(trimmed);

    if stripped.is_empty() {
        return Err("LLM output was empty after sanitization".to_string());
    }

    Ok(stripped.to_string())
}

fn strip_surrounding_quotes(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return s[1..s.len() - 1].trim();
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_known_models() {
        assert!(validate_model("standard").is_ok());
    }

    #[test]
    fn validate_rejects_unknown() {
        assert!(validate_model("light").is_err());
        assert!(validate_model("large").is_err());
        assert!(validate_model("../etc/passwd").is_err());
        assert!(validate_model("").is_err());
    }

    #[test]
    fn filenames_are_stable() {
        assert_eq!(model_filename("standard").unwrap(), "gemma-4-E4B-it-Q4_K_M.gguf");
    }

    #[test]
    fn urls_are_https_and_huggingface() {
        let std = model_download_url("standard").unwrap();
        assert!(std.starts_with("https://huggingface.co/"));
    }

    #[test]
    fn medical_model_validates() {
        assert!(validate_medical_model("biomistral-7b-q5").is_ok());
        assert!(validate_medical_model("standard").is_err());
        assert!(validate_medical_model("../etc/passwd").is_err());
    }

    #[test]
    fn medical_filename_is_stable() {
        assert_eq!(
            medical_model_filename("biomistral-7b-q5").unwrap(),
            "BioMistral-7B.Q5_K_M.gguf"
        );
    }

    #[test]
    fn medical_url_is_https_and_huggingface() {
        let url = medical_model_download_url("biomistral-7b-q5").unwrap();
        assert!(url.starts_with("https://huggingface.co/"));
        assert!(url.ends_with(".gguf"));
    }

    #[test]
    fn role_ports_are_distinct() {
        assert_ne!(LlmRole::Cleanup.port(), LlmRole::Medical.port());
    }

    #[test]
    fn extract_strips_think_block() {
        let input = "<think>\nReasoning here\n</think>\nThe cleaned text.";
        assert_eq!(extract_clean_or_fail(input).unwrap(), "The cleaned text.");
    }

    #[test]
    fn extract_passes_clean_text() {
        let input = "Hello world.";
        assert_eq!(extract_clean_or_fail(input).unwrap(), "Hello world.");
    }

    #[test]
    fn extract_strips_empty_think() {
        let input = "<think>\n</think>\nHey Sarah.";
        assert_eq!(extract_clean_or_fail(input).unwrap(), "Hey Sarah.");
    }

    #[test]
    fn extract_strips_channel_thought() {
        let input = "<|channel>thought\nstep one\nstep two<channel|>\nFinal cleaned text.";
        assert_eq!(extract_clean_or_fail(input).unwrap(), "Final cleaned text.");
    }

    #[test]
    fn extract_fails_on_leaked_thinking_process() {
        let input = "Thinking Process:\n1. Analyze input\n2. Apply rules\nFinal text here.";
        assert!(extract_clean_or_fail(input).is_err());
    }

    #[test]
    fn extract_fails_on_unclosed_think() {
        let input = "<think>\nUnclosed reasoning that bleeds into the output";
        assert!(extract_clean_or_fail(input).is_err());
    }

    #[test]
    fn extract_strips_surrounding_quotes() {
        let input = "\"This is the cleaned text.\"";
        assert_eq!(extract_clean_or_fail(input).unwrap(), "This is the cleaned text.");
    }

    #[test]
    fn extract_keeps_internal_quotes() {
        let input = "She said \"hello\" to him.";
        assert_eq!(extract_clean_or_fail(input).unwrap(), "She said \"hello\" to him.");
    }

    #[test]
    fn extract_fails_on_empty_after_strip() {
        let input = "<think>\nonly reasoning\n</think>";
        assert!(extract_clean_or_fail(input).is_err());
    }

    #[test]
    fn extract_fails_on_user_meta_reference() {
        let input = "The user wants me to clean up a transcript. Here is the text.";
        assert!(extract_clean_or_fail(input).is_err());
    }

    #[test]
    fn extract_fails_on_rules_checklist() {
        let input = "Rules Checklist:\n1. Done\n2. Done\nFinal text.";
        assert!(extract_clean_or_fail(input).is_err());
    }
}
