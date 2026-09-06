//! Microphone TCC via AVFoundation.
//!
//! cpal opens a CoreAudio HAL stream. Under App Sandbox (MAS / TestFlight)
//! that `play()` can succeed and still deliver digital silence when
//! `kTCCServiceMicrophone` is not determined or denied — and HAL does not
//! reliably show the system prompt. Requesting
//! `AVCaptureDevice.requestAccess(for: .audio)` is what actually drives TCC
//! and is required before we treat a live overlay as a real recording.

use crate::dictation_error::{self, UserError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicAuth {
    NotDetermined,
    Restricted,
    Denied,
    Authorized,
}

#[cfg(target_os = "macos")]
mod native {
    use super::MicAuth;

    unsafe extern "C" {
        fn mabel_mic_authorization_status() -> i32;
        fn mabel_mic_request_access() -> i32;
    }

    pub fn status() -> MicAuth {
        match unsafe { mabel_mic_authorization_status() } {
            1 => MicAuth::Restricted,
            2 => MicAuth::Denied,
            3 => MicAuth::Authorized,
            _ => MicAuth::NotDetermined,
        }
    }

    pub fn request_access() -> bool {
        unsafe { mabel_mic_request_access() == 1 }
    }
}

#[cfg(not(target_os = "macos"))]
mod native {
    use super::MicAuth;

    pub fn status() -> MicAuth {
        MicAuth::Authorized
    }

    pub fn request_access() -> bool {
        true
    }
}

pub fn status() -> MicAuth {
    native::status()
}

/// Prompt if needed. Fail closed when the user has denied or restricted mic
/// access. Safe to call from a worker thread (must not run on the AppKit
/// main thread — the ObjC helper waits on a semaphore).
pub fn ensure_granted() -> Result<(), UserError> {
    match status() {
        MicAuth::Authorized => Ok(()),
        MicAuth::Denied | MicAuth::Restricted => {
            crate::system_ui::open_microphone_settings();
            Err(dictation_error::mic_denied())
        }
        MicAuth::NotDetermined => {
            if native::request_access() {
                Ok(())
            } else {
                crate::system_ui::open_microphone_settings();
                Err(dictation_error::mic_denied())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_macos_ci_treats_mic_as_granted() {
        if cfg!(not(target_os = "macos")) {
            assert_eq!(status(), MicAuth::Authorized);
            assert!(ensure_granted().is_ok());
        }
    }
}
