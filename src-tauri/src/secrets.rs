use keyring::Entry;
use std::sync::RwLock;

const SERVICE: &str = "com.mabel.app";
const GROQ_KEY_ACCOUNT: &str = "groq_api_key";

/// In-process cache of the Groq key. First access touches the keychain (and on
/// dev builds with changing signatures, prompts the user). Subsequent calls
/// read from this cache, so a dictation flurry doesn't trigger a flurry of
/// keychain prompts. Updated whenever set_groq_key writes a new value.
static CACHED: RwLock<Option<String>> = RwLock::new(None);

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE, GROQ_KEY_ACCOUNT).map_err(|e| format!("Keychain entry error: {}", e))
}

pub fn get_groq_key() -> Result<String, String> {
    // Dev override: setting MABEL_GROQ_KEY in the shell skips the keychain
    // entirely. Useful when running `npm run tauri dev` because Rust edits
    // change the binary signature each rebuild, which makes macOS treat every
    // dev session as a new app and re-prompt for keychain access. In a signed
    // production install the keychain prompt fires once per machine and
    // "Always Allow" persists forever.
    if let Ok(env_key) = std::env::var("MABEL_GROQ_KEY") {
        if !env_key.is_empty() {
            return Ok(env_key);
        }
    }
    if let Some(cached) = CACHED.read().unwrap().clone() {
        if !cached.is_empty() {
            return Ok(cached);
        }
    }
    let key = keychain_password_to_groq_key(entry()?.get_password())?;
    *CACHED.write().unwrap() = Some(key.clone());
    Ok(key)
}

/// Fail closed: a missing, empty, or unreadable keychain item is an error.
/// Never `Ok("")` — empty-from-auth must not look like a successful key read.
pub(crate) fn keychain_password_to_groq_key(
    result: Result<String, keyring::Error>,
) -> Result<String, String> {
    match result {
        Ok(s) if !s.trim().is_empty() => Ok(s),
        Ok(_) => Err("Groq API key not set".to_string()),
        Err(keyring::Error::NoEntry) => Err("Groq API key not set".to_string()),
        Err(e) => Err(format!("Keychain read error: {}", e)),
    }
}

/// Probe whether a key is stored without holding it in cache. Used by the
/// Settings UI to decide whether to show "Saved". On the first call after
/// install this still triggers the macOS keychain prompt (unavoidable —
/// macOS requires consent before disclosing any keychain item, including its
/// existence in the form we have here).
pub fn has_groq_key() -> bool {
    matches!(entry().and_then(|e| e.get_password().map_err(|err| err.to_string())), Ok(s) if !s.is_empty())
}

pub fn set_groq_key(value: &str) -> Result<(), String> {
    let e = entry()?;
    if value.is_empty() {
        let res = match e.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(format!("Keychain delete error: {}", err)),
        };
        if res.is_ok() {
            *CACHED.write().unwrap() = None;
        }
        res
    } else {
        let res = e
            .set_password(value)
            .map_err(|err| format!("Keychain write error: {}", err));
        if res.is_ok() {
            *CACHED.write().unwrap() = Some(value.to_string());
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_or_empty_keychain_entry_is_err_not_empty_ok() {
        assert!(keychain_password_to_groq_key(Err(keyring::Error::NoEntry)).is_err());
        assert!(keychain_password_to_groq_key(Ok(String::new())).is_err());
        assert!(keychain_password_to_groq_key(Ok("   ".into())).is_err());
        assert_eq!(
            keychain_password_to_groq_key(Ok("gsk_test".into())).unwrap(),
            "gsk_test"
        );
    }

    #[test]
    fn keychain_platform_error_stays_err() {
        let err = keychain_password_to_groq_key(Err(keyring::Error::Invalid(
            "keychain".into(),
            "default keychain could not be found".into(),
        )))
        .unwrap_err();
        assert!(err.contains("Keychain read error"), "{err}");
        assert!(err.contains("default keychain could not be found"), "{err}");
    }
}
