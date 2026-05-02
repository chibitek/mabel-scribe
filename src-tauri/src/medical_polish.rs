// Stage 2: medical-terminology polish.
//
// Pipeline position: runs AFTER the cleanup pass (Gemma) returns a polished
// dictation block. BioMistral 7B (medical-domain fine-tune of Mistral 7B,
// Apache 2.0) inspects the cleaned text plus the user's medical glossary and
// returns a diff: a list of {find, replace, evidence} objects. We apply the
// fixes via plain string substitution and return the result.
//
// Why a diff instead of a full rewrite:
//   - Output token count drops ~10x for a typical block, which is the
//     dominant latency lever after prompt caching.
//   - Eliminates the failure mode where a medical fine-tune "improves" the
//     text by adding clinical content not present in the input.
//   - Falls naturally onto an accept/reject UI when we add one.
//
// Constraints applied here:
//   - GBNF grammar forces the response into the diff schema. The model
//     cannot emit prose.
//   - Greedy decoding (temp 0). Determinism matters more than creativity.
//   - Transcript-evidence requested in-prompt. The grammar makes evidence
//     optional so a model that omits it still produces a parseable response,
//     but the prompt asks for it so we have a record per fix.
//   - Apply fixes left-to-right; drop any edit whose `find` is not present
//     verbatim in the current text state.

use std::time::Duration;

use crate::llm::LlmRole;

const MEDICAL_TIMEOUT: Duration = Duration::from_secs(20);

/// llama.cpp GBNF for the diff-array output. Allows an empty array, otherwise
/// each element must be an object with at least `find` and `replace`.
const DIFF_GRAMMAR: &str = r#"root ::= "[" ws ("]" | edit (ws "," ws edit)* ws "]")
edit ::= "{" ws "\"find\":" ws string ws "," ws "\"replace\":" ws string (ws "," ws "\"evidence\":" ws string)? ws "}"
string ::= "\"" ([^"\\] | "\\" ["\\bfnrt/])* "\""
ws ::= [ \t\n]*
"#;

const SYSTEM_PROMPT: &str = "You are a medical terminology editor. Your only job is to fix misspelled or non-standard medical terms in a transcript that has already been cleaned for grammar and punctuation.\n\nRules:\n- Output a JSON array of {\"find\", \"replace\", \"evidence\"} objects.\n- \"find\" must be a substring of the input transcript, copied verbatim.\n- \"replace\" is the corrected medical term.\n- \"evidence\" is the same substring as \"find\" or a slightly larger surrounding phrase from the transcript.\n- Only fix medical terms: drug names, anatomy, conditions, procedures, common medical abbreviations.\n- Do not fix grammar, punctuation, or non-medical words.\n- Do not add new content. Every fix must be a swap of words already present in the transcript.\n- If a glossary is provided, prefer those spellings.\n- If nothing needs fixing, output [].\n- Output ONLY the JSON array. No prose, no explanation, no preamble.";

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
    grammar: &'a str,
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

#[derive(serde::Deserialize, Debug, PartialEq)]
struct Edit {
    find: String,
    replace: String,
    #[serde(default)]
    #[allow(dead_code)] // retained for audit; not consumed by the apply step yet
    evidence: Option<String>,
}

/// Heuristic gate: the polish pass is only useful when there's enough text to
/// plausibly contain medical terminology. Skipping micro-utterances dodges the
/// per-call latency on casual dictation.
fn should_polish(cleaned: &str) -> bool {
    cleaned.trim().chars().count() >= 30
}

fn build_user_prompt(cleaned: &str, glossary: &[String]) -> String {
    let glossary_block = if glossary.is_empty() {
        String::new()
    } else {
        format!(
            "Glossary (prefer these spellings):\n{}\n\n",
            glossary.join(", ")
        )
    };

    let examples = "Example 1\n\
Transcript: \"Patient takes amoxicilin 500 mg twice daily.\"\n\
Output: [{\"find\":\"amoxicilin\",\"replace\":\"amoxicillin\",\"evidence\":\"amoxicilin 500 mg\"}]\n\n\
Example 2\n\
Transcript: \"BP 130 over 85, no chest pain or sob.\"\n\
Output: [{\"find\":\"sob\",\"replace\":\"shortness of breath\",\"evidence\":\"chest pain or sob\"}]\n\n\
Example 3\n\
Transcript: \"Reviewed labs and discussed plan.\"\n\
Output: []\n\n";

    format!(
        "{}{}Transcript: {}\nOutput:",
        glossary_block,
        examples,
        json_escape(cleaned)
    )
}

/// Wrap a transcript in JSON-quoted form so it appears as a literal in the
/// prompt the model sees. We don't use `serde_json` here because we want full
/// control over what goes into the prompt body.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Calls the running medical-polish llama-server on cleaned text. Returns the
/// polished text on success or Err on any failure (timeout, parse error,
/// server not running). The caller treats Err as "stage 2 not available; use
/// stage 1 output as final".
pub async fn polish(cleaned: &str, glossary: &[String]) -> Result<String, String> {
    let trimmed = cleaned.trim();
    if !should_polish(trimmed) {
        return Ok(cleaned.to_string());
    }

    let req = ChatRequest {
        messages: vec![
            ChatMessage {
                role: "system",
                content: SYSTEM_PROMPT.to_string(),
            },
            ChatMessage {
                role: "user",
                content: build_user_prompt(trimmed, glossary),
            },
        ],
        temperature: 0.0,
        max_tokens: 256,
        stream: false,
        grammar: DIFF_GRAMMAR,
    };

    let client = reqwest::Client::builder()
        .timeout(MEDICAL_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "http://127.0.0.1:{}/v1/chat/completions",
        LlmRole::Medical.port()
    );
    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .map_err(|e| format!("Medical polish request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Medical polish returned status {}", resp.status()));
    }

    let parsed: ChatResponse = resp
        .json()
        .await
        .map_err(|e| format!("Medical polish response parse failed: {}", e))?;

    let raw = parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| "Medical polish response had no choices".to_string())?;

    let edits: Vec<Edit> = serde_json::from_str(raw.trim())
        .map_err(|e| format!("Medical polish JSON parse failed: {}: {:?}", e, raw))?;

    Ok(apply_edits(cleaned, &edits))
}

/// Apply each edit's find/replace once, in order. Skips any edit whose `find`
/// is not present in the current state of the text. If the model wants the
/// same substring rewritten in multiple places, it must emit one edit per
/// occurrence — this preserves the principle that every fix is an explicit
/// model decision.
fn apply_edits(input: &str, edits: &[Edit]) -> String {
    let mut current = input.to_string();
    for edit in edits {
        if edit.find.is_empty() {
            continue;
        }
        if let Some(idx) = current.find(&edit.find) {
            current.replace_range(idx..idx + edit.find.len(), &edit.replace);
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_short_text() {
        assert!(!should_polish(""));
        assert!(!should_polish("   "));
        assert!(!should_polish("Hi."));
        assert!(should_polish(
            "Patient presents with shortness of breath today."
        ));
    }

    #[test]
    fn applies_simple_edit() {
        let edits = vec![Edit {
            find: "amoxicilin".to_string(),
            replace: "amoxicillin".to_string(),
            evidence: None,
        }];
        assert_eq!(
            apply_edits("Take amoxicilin 500 mg.", &edits),
            "Take amoxicillin 500 mg."
        );
    }

    #[test]
    fn skips_edit_when_find_missing() {
        let edits = vec![Edit {
            find: "tylenol".to_string(),
            replace: "acetaminophen".to_string(),
            evidence: None,
        }];
        assert_eq!(
            apply_edits("Patient took ibuprofen.", &edits),
            "Patient took ibuprofen."
        );
    }

    #[test]
    fn applies_first_occurrence_only_per_edit() {
        let edits = vec![Edit {
            find: "sob".to_string(),
            replace: "shortness of breath".to_string(),
            evidence: None,
        }];
        assert_eq!(
            apply_edits("sob then sob again", &edits),
            "shortness of breath then sob again"
        );
    }

    #[test]
    fn skips_empty_find() {
        let edits = vec![Edit {
            find: "".to_string(),
            replace: "anything".to_string(),
            evidence: None,
        }];
        assert_eq!(apply_edits("Original text.", &edits), "Original text.");
    }

    #[test]
    fn applies_multiple_edits_in_order() {
        let edits = vec![
            Edit {
                find: "hipertension".to_string(),
                replace: "hypertension".to_string(),
                evidence: None,
            },
            Edit {
                find: "diabetis".to_string(),
                replace: "diabetes".to_string(),
                evidence: None,
            },
        ];
        assert_eq!(
            apply_edits("History of hipertension and diabetis.", &edits),
            "History of hypertension and diabetes."
        );
    }

    #[test]
    fn empty_edits_returns_input_unchanged() {
        assert_eq!(apply_edits("Original.", &[]), "Original.");
    }

    #[test]
    fn user_prompt_omits_glossary_block_when_empty() {
        let p = build_user_prompt("Hello world.", &[]);
        assert!(!p.contains("Glossary"));
    }

    #[test]
    fn user_prompt_includes_glossary_terms() {
        let p = build_user_prompt(
            "Hello world.",
            &["amoxicillin".to_string(), "lisinopril".to_string()],
        );
        assert!(p.contains("Glossary"));
        assert!(p.contains("amoxicillin"));
        assert!(p.contains("lisinopril"));
    }

    #[test]
    fn json_escape_quotes_and_backslashes() {
        assert_eq!(json_escape("plain"), "\"plain\"");
        assert_eq!(json_escape("a\"b"), "\"a\\\"b\"");
        assert_eq!(json_escape("a\\b"), "\"a\\\\b\"");
        assert_eq!(json_escape("line1\nline2"), "\"line1\\nline2\"");
    }

    #[test]
    fn parses_diff_with_evidence() {
        let raw = r#"[{"find":"sob","replace":"shortness of breath","evidence":"chest pain or sob"}]"#;
        let edits: Vec<Edit> = serde_json::from_str(raw).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].find, "sob");
        assert_eq!(edits[0].replace, "shortness of breath");
        assert_eq!(
            edits[0].evidence.as_deref(),
            Some("chest pain or sob")
        );
    }

    #[test]
    fn parses_diff_without_evidence() {
        let raw = r#"[{"find":"sob","replace":"shortness of breath"}]"#;
        let edits: Vec<Edit> = serde_json::from_str(raw).unwrap();
        assert_eq!(edits.len(), 1);
        assert!(edits[0].evidence.is_none());
    }

    #[test]
    fn parses_empty_array() {
        let edits: Vec<Edit> = serde_json::from_str("[]").unwrap();
        assert!(edits.is_empty());
    }
}
