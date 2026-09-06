// LLM cleanup pass: runs Whisper output through a local LLM (llama.cpp server)
// to remove fillers, fix punctuation, and normalize numbers/proper nouns.
//
// Architecture: spawn bundled `llama-server` (src-tauri/llama-runtime/, vendored
// from the official llama.cpp macOS-arm64 release) bound to 127.0.0.1, hold
// the child handle in app state, and POST to its OpenAI-compatible
// /v1/chat/completions endpoint per cleanup. The server keeps the model warm
// between calls so per-cleanup latency is just generation, not model load.
// After IDLE_UNLOAD with no activity the process is killed so ~5 GB is not
// pinned. Homebrew is a last-resort fallback only.
//
// Two model tiers, downloaded lazily into the app config dir alongside the
// Whisper models:
//   - "light"    → SmolLM3-3B Q4_K_M  (~1.8 GB)  — fast, weaker on numbers/terms
//   - "standard" → Gemma 4 E4B Q4_K_M (~5.0 GB)  — better quality, default

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

const ALLOWED_MODELS: &[&str] = &["standard"];

/// Loopback port we bind llama-server to. Picked to be high and uncommon. If
/// this collides with another process the server will fail to start and we'll
/// fall back to the rules-only cleanup path.
pub const SERVER_PORT: u16 = 18745;

/// Hard cap on how long we wait for the model to load before giving up on the
/// cleanup call. Cold model load on M1 base for Gemma 4 E4B is ~3s.
const READY_TIMEOUT: Duration = Duration::from_secs(20);

/// Per-cleanup HTTP timeout. Generation alone is sub-second on M-series; pad
/// generously so a slow Intel Mac doesn't drop the request mid-flight.
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(15);

/// Unload the ~5 GB Gemma weights after this much idle time so they are not
/// pinned forever. The next cleanup call cold-starts the server again.
pub const IDLE_UNLOAD: Duration = Duration::from_secs(5 * 60);

pub fn validate_model(model: &str) -> Result<&str, String> {
    if ALLOWED_MODELS.contains(&model) {
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

/// Holds the running llama-server process handle plus the model it was started
/// with. We keep both so we can detect a model change in settings and respawn.
pub struct LlmServer {
    inner: Mutex<Option<RunningServer>>,
    last_used: Mutex<Option<Instant>>,
}

struct RunningServer {
    child: Child,
    model: String,
}

/// Candidate paths for the bundled / override / Homebrew llama-server.
/// Bundled copies live next to their own dylibs (see scripts/vendor-llama-server.sh)
/// so they do not collide with Whisper's ggml Frameworks.
pub fn llama_server_candidates(app: Option<&AppHandle>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(p) = std::env::var("MABEL_LLAMA_SERVER") {
        if !p.is_empty() {
            candidates.push(PathBuf::from(p));
        }
    }

    if let Some(app) = app {
        if let Ok(res) = app.path().resource_dir() {
            candidates.push(res.join("llama-runtime").join("llama-server"));
            candidates.push(res.join("llama-server"));
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // Production .app: Contents/MacOS/../Resources/llama-runtime/llama-server
            candidates.push(dir.join("../Resources/llama-runtime/llama-server"));
            // Sidecar-style next to the main binary
            candidates.push(dir.join("llama-server"));
            // `tauri dev`: target/debug/../../llama-runtime/llama-server
            candidates.push(dir.join("../../llama-runtime/llama-server"));
        }
    }

    candidates.push(PathBuf::from("/opt/homebrew/bin/llama-server"));
    candidates.push(PathBuf::from("/usr/local/bin/llama-server"));
    candidates
}

/// Resolves the path to `llama-server` for spawning.
///
/// Order: `MABEL_LLAMA_SERVER`, bundled llama-runtime (shipped with the app),
/// then Homebrew as a last-resort fallback for developers.
fn resolve_llama_server_path(app: Option<&AppHandle>) -> Option<PathBuf> {
    llama_server_candidates(app).into_iter().find(|p| p.exists())
}

pub fn runtime_available() -> bool {
    resolve_llama_server_path(None).is_some()
}

impl LlmServer {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
            last_used: Mutex::new(None),
        }
    }

    pub fn touch(&self) {
        *self.last_used.lock().unwrap() = Some(Instant::now());
    }

    /// Kills the server when it has been idle long enough. Idempotent.
    pub fn stop_if_idle(&self, idle_for: Duration) {
        let last = *self.last_used.lock().unwrap();
        let Some(t) = last else { return };
        if t.elapsed() < idle_for {
            return;
        }
        println!(
            "[Mabel] Unloading llama-server after {:?} idle",
            t.elapsed()
        );
        self.stop();
        *self.last_used.lock().unwrap() = None;
    }

    /// Returns true if the server is running with the requested model.
    pub fn is_ready_for(&self, model: &str) -> bool {
        let guard = self.inner.lock().unwrap();
        guard.as_ref().map(|s| s.model == model).unwrap_or(false)
    }

    /// Starts llama-server with the given model. If a server is already running
    /// with a different model, kills it first. Caller is responsible for
    /// ensuring the model file exists at `model_path`.
    pub async fn start(
        &self,
        app: &AppHandle,
        model: &str,
        model_path: &PathBuf,
    ) -> Result<(), String> {
        if self.is_ready_for(model) {
            self.touch();
            return Ok(());
        }
        self.stop();

        if !model_path.exists() {
            return Err(format!("LLM model not found: {:?}", model_path));
        }

        let bin = resolve_llama_server_path(Some(app))
            .ok_or_else(|| {
                "llama-server binary not found (bundled runtime missing; set MABEL_LLAMA_SERVER or install llama.cpp)"
                    .to_string()
            })?;

        println!(
            "[Mabel] Starting llama-server ({:?}) for {} ({:?})",
            bin, model, model_path
        );

        let mut cmd = Command::new(&bin);
        // Official binaries resolve @rpath dylibs from @loader_path (the
        // directory containing llama-server). Setting cwd matches that layout
        // for any relative sidecar files such as ggml-metal-tuning.
        if let Some(dir) = bin.parent() {
            cmd.current_dir(dir);
        }
        let child = cmd
            .args([
                "-m",
                model_path.to_str().unwrap(),
                "--host",
                "127.0.0.1",
                "--port",
                &SERVER_PORT.to_string(),
                // 2K context is plenty for dictation utterances. Larger context
                // costs RAM at load.
                "-c",
                "2048",
                // Required to apply the model's chat template.
                "--jinja",
                // Suppress reasoning tokens. Both Gemma 4 and SmolLM3 emit them
                // by default; for cleanup we want immediate text-only answers.
                "--reasoning-budget",
                "0",
                // Offload all layers to Metal on macOS. On non-Metal hosts the
                // flag is silently ignored.
                "-ngl",
                "99",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn llama-server: {}", e))?;

        {
            let mut guard = self.inner.lock().unwrap();
            *guard = Some(RunningServer {
                child,
                model: model.to_string(),
            });
        }

        // Poll /health until ready or timeout. The server returns 200 once the
        // model is fully loaded.
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/health", SERVER_PORT);
        let deadline = std::time::Instant::now() + READY_TIMEOUT;
        loop {
            if std::time::Instant::now() >= deadline {
                self.stop();
                return Err("llama-server failed to become ready in time".to_string());
            }
            match client.get(&url).timeout(Duration::from_millis(500)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    println!("[Mabel] llama-server ready");
                    self.touch();
                    return Ok(());
                }
                _ => tokio::time::sleep(Duration::from_millis(250)).await,
            }
        }
    }

    /// Kills the running server if any. Idempotent.
    pub fn stop(&self) {
        let mut guard = self.inner.lock().unwrap();
        if let Some(mut server) = guard.take() {
            println!("[Mabel] Stopping llama-server");
            let _ = server.child.kill();
            let _ = server.child.wait();
        }
    }
}

/// Run local Gemma Polish after the rules pass, or return the rules text.
///
/// This step is on-device only (llama-server on 127.0.0.1). It never calls
/// Groq / Nexus / Coach. Live Polish requires Pro (`polish::effective_mode`).
/// Any Gemma failure falls back to `rule_cleaned` so we never invent
/// facts or paste meaning-expanded text.
pub async fn polish_or_rules(
    app: &AppHandle,
    server: &LlmServer,
    settings: &crate::settings::Settings,
    app_dir: &PathBuf,
    rule_cleaned: String,
) -> String {
    if rule_cleaned.is_empty() {
        return rule_cleaned;
    }
    let mode = crate::polish::effective_mode(&settings.polish_mode);
    if !crate::polish::is_live(&mode) {
        return rule_cleaned;
    }
    println!(
        "[Mabel] Polish {} requested (chars={})",
        mode,
        rule_cleaned.chars().count()
    );
    let t0 = std::time::Instant::now();
    let llm_result = match model_filename(&settings.llm_model) {
        Ok(name) => {
            let model_path = app_dir.join(name);
            ensure_and_cleanup_mode(
                app,
                server,
                &settings.llm_model,
                &model_path,
                &rule_cleaned,
                &mode,
            )
            .await
        }
        Err(e) => Err(e),
    };
    match llm_result {
        Ok(s) if !s.is_empty() => {
            println!(
                "[Mabel] Polish succeeded ({:?}, chars={})",
                t0.elapsed(),
                s.chars().count()
            );
            s
        }
        Ok(empty) => {
            println!(
                "[Mabel] Polish returned empty ({:?}, chars={}); falling back to rules",
                t0.elapsed(),
                empty.chars().count()
            );
            rule_cleaned
        }
        Err(e) => {
            eprintln!(
                "[Mabel] Polish failed ({:?}), using rules (never invent): {}",
                t0.elapsed(),
                e
            );
            rule_cleaned
        }
    }
}

/// Starts the server if needed, then runs cleanup. Used by the dictation
/// path so an idle-unload does not silently drop the next AI cleanup.
pub async fn ensure_and_cleanup(
    app: &AppHandle,
    server: &LlmServer,
    model: &str,
    model_path: &PathBuf,
    text: &str,
) -> Result<String, String> {
    ensure_and_cleanup_mode(app, server, model, model_path, text, crate::polish::MODE_CASUAL).await
}

/// Same as `ensure_and_cleanup`, with a Polish register preset.
pub async fn ensure_and_cleanup_mode(
    app: &AppHandle,
    server: &LlmServer,
    model: &str,
    model_path: &PathBuf,
    text: &str,
    polish_mode: &str,
) -> Result<String, String> {
    server.start(app, model, model_path).await?;
    server.touch();
    cleanup_with_llm_mode(text, polish_mode).await
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

/// Calls the running llama-server to clean up a transcript. The server must
/// already be started — caller should ensure that via `LlmServer::start` (which
/// is fast on subsequent calls because it short-circuits when already running).
///
/// On any failure, returns an Err and the caller should fall back to the
/// rules-only cleanup output. Cleanup is best-effort; never block paste on it.
pub async fn cleanup_with_llm(text: &str) -> Result<String, String> {
    cleanup_with_llm_mode(text, crate::polish::MODE_CASUAL).await
}

pub async fn cleanup_with_llm_mode(text: &str, polish_mode: &str) -> Result<String, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    let req = ChatRequest {
        messages: vec![
            ChatMessage {
                role: "system",
                content: crate::polish::system_prompt(polish_mode),
            },
            ChatMessage {
                role: "user",
                content: format!("{}{}", crate::polish::user_prompt_prefix(), trimmed),
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

    let url = format!("http://127.0.0.1:{}/v1/chat/completions", SERVER_PORT);
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
    fn bundled_candidates_include_app_resources_and_homebrew() {
        let paths = llama_server_candidates(None);
        let rendered: Vec<String> = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
        assert!(rendered.iter().any(|p| p.ends_with("llama-runtime/llama-server")
            || p.ends_with("Resources/llama-runtime/llama-server")
            || p.contains("llama-runtime")));
        assert!(rendered.iter().any(|p| p == "/opt/homebrew/bin/llama-server"));
        assert!(rendered.iter().any(|p| p == "/usr/local/bin/llama-server"));
    }

    #[test]
    fn idle_unload_without_server_is_a_noop() {
        let server = LlmServer::new();
        server.stop_if_idle(Duration::from_secs(0));
        assert!(!server.is_ready_for("standard"));
    }

    #[test]
    fn touch_prevents_immediate_idle_unload() {
        let server = LlmServer::new();
        server.touch();
        server.stop_if_idle(IDLE_UNLOAD);
        // No child was started; touch only records activity.
        assert!(server.last_used.lock().unwrap().is_some());
    }

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

    #[test]
    fn polish_prompts_are_the_gemma_cleanup_path() {
        let casual = crate::polish::system_prompt("casual");
        assert!(casual.contains("Never invent facts"));
        assert!(casual.contains("Never expand meaning"));
        assert!(crate::polish::user_prompt_prefix().contains("Never invent facts"));
        assert!(format!("http://127.0.0.1:{SERVER_PORT}").contains("127.0.0.1"));
    }
}
