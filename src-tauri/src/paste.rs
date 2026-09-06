use arboard::{Clipboard, ImageData};

use crate::system_ui;

pub fn paste_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    let previous = ClipboardBackup::capture(&mut clipboard);
    clipboard.set_text(text).map_err(|e| e.to_string())?;

    std::thread::sleep(std::time::Duration::from_millis(50));

    let osa_err = match paste_via_osascript() {
        Ok(()) => {
            std::thread::sleep(std::time::Duration::from_millis(150));
            previous.restore(&mut clipboard);
            return Ok(());
        }
        Err(e) => e,
    };

    // MAS sandbox can block System Events without a temporary Apple Events
    // exception. Accessibility-trusted CGEvent Cmd+V is the fallback so
    // record→stop→text still lands when the user granted AX.
    if system_ui::is_accessibility_trusted(false) {
        if paste_via_cgevent().is_ok() {
            std::thread::sleep(std::time::Duration::from_millis(150));
            previous.restore(&mut clipboard);
            return Ok(());
        }
    }

    previous.restore(&mut clipboard);
    Err(osa_err)
}

fn paste_via_osascript() -> Result<(), String> {
    let output = std::process::Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to keystroke "v" using command down"#,
        ])
        .output()
        .map_err(|e| format!("Failed to simulate paste: {}", e))?;

    if !output.status.success() {
        return Err(format_command_failure("AppleScript paste", &output));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn paste_via_cgevent() -> Result<(), String> {
    use std::ffi::c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceCreate(state_id: u32) -> *mut c_void;
        fn CGEventCreateKeyboardEvent(
            source: *mut c_void,
            virtual_key: u16,
            key_down: bool,
        ) -> *mut c_void;
        fn CGEventSetFlags(event: *mut c_void, flags: u64);
        fn CGEventPost(tap: u32, event: *mut c_void);
        fn CFRelease(cf: *const c_void);
    }

    const HID_SYSTEM_STATE: u32 = 1;
    const HID_TAP: u32 = 0;
    const COMMAND: u64 = 0x0010_0000; // kCGEventFlagMaskCommand
    const KEY_V: u16 = 9;

    unsafe {
        let source = CGEventSourceCreate(HID_SYSTEM_STATE);
        if source.is_null() {
            return Err("CGEventSourceCreate failed".into());
        }
        let down = CGEventCreateKeyboardEvent(source, KEY_V, true);
        let up = CGEventCreateKeyboardEvent(source, KEY_V, false);
        if down.is_null() || up.is_null() {
            if !down.is_null() {
                CFRelease(down);
            }
            if !up.is_null() {
                CFRelease(up);
            }
            CFRelease(source);
            return Err("CGEventCreateKeyboardEvent failed".into());
        }
        CGEventSetFlags(down, COMMAND);
        CGEventSetFlags(up, COMMAND);
        CGEventPost(HID_TAP, down);
        CGEventPost(HID_TAP, up);
        CFRelease(down);
        CFRelease(up);
        CFRelease(source);
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn paste_via_cgevent() -> Result<(), String> {
    Err("CGEvent paste is macOS-only".into())
}

enum ClipboardBackup {
    Text(String),
    Image(ImageData<'static>),
    Empty,
}

impl ClipboardBackup {
    fn capture(clipboard: &mut Clipboard) -> Self {
        if let Ok(text) = clipboard.get_text() {
            return Self::Text(text);
        }
        if let Ok(image) = clipboard.get_image() {
            return Self::Image(image.to_owned_img());
        }
        Self::Empty
    }

    fn restore(self, clipboard: &mut Clipboard) {
        match self {
            Self::Text(text) => {
                let _ = clipboard.set_text(text);
            }
            Self::Image(image) => {
                let _ = clipboard.set_image(image);
            }
            Self::Empty => {
                let _ = clipboard.clear();
            }
        }
    }
}

pub fn press_return() -> Result<(), String> {
    let output = std::process::Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to key code 36"#,
        ])
        .output()
        .map_err(|e| format!("Failed to press Return: {}", e))?;
    if !output.status.success() {
        return Err(format_command_failure("AppleScript Return", &output));
    }
    Ok(())
}

fn format_command_failure(action: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        "no command output".to_string()
    };
    format!("{} failed (status {}): {}", action, output.status, detail)
}

/// If `press_enter` is on and the transcription ends with a "press enter" /
/// "new line" command phrase, strip the phrase and return (cleaned_text, true).
/// Otherwise returns (text_unchanged, false).
pub fn extract_press_enter_command(text: &str, enabled: bool) -> (String, bool) {
    if !enabled {
        return (text.to_string(), false);
    }
    let trimmed = text.trim_end_matches(|c: char| c.is_whitespace() || matches!(c, '.' | '!' | '?' | ','));
    let lower = trimmed.to_lowercase();
    for phrase in ["press enter", "press return", "new line", "newline"] {
        if let Some(idx) = lower.rfind(phrase) {
            // Phrase must be at the end (allowing for trailing punctuation we already stripped).
            if idx + phrase.len() == lower.len() {
                let cleaned = trimmed[..idx].trim_end().to_string();
                return (cleaned, true);
            }
        }
    }
    (text.to_string(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_press_enter() {
        let (out, hit) = extract_press_enter_command("send the email press enter", true);
        assert!(hit);
        assert_eq!(out, "send the email");
    }

    #[test]
    fn handles_trailing_punctuation() {
        let (out, hit) = extract_press_enter_command("ok press enter.", true);
        assert!(hit);
        assert_eq!(out, "ok");
    }

    #[test]
    fn ignores_when_disabled() {
        let (out, hit) = extract_press_enter_command("hello press enter", false);
        assert!(!hit);
        assert_eq!(out, "hello press enter");
    }

    #[test]
    fn ignores_phrase_in_middle() {
        let (out, hit) = extract_press_enter_command("press enter to confirm please", true);
        assert!(!hit);
        assert_eq!(out, "press enter to confirm please");
    }
}
