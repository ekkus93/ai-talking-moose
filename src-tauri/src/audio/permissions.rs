use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicrophonePermissionState {
    NotRequested,
    Granted,
    Denied,
    Unavailable,
}

impl MicrophonePermissionState {
    pub fn is_granted(self) -> bool {
        self == Self::Granted
    }
}

#[cfg(target_os = "macos")]
fn macos_microphone_permission_state() -> MicrophonePermissionState {
    use av_foundation::capture_device::{
        AVAuthorizationStatusAuthorized, AVAuthorizationStatusDenied,
        AVAuthorizationStatusNotDetermined, AVAuthorizationStatusRestricted, AVCaptureDevice,
    };
    use av_foundation::media_format::AVMediaTypeAudio;

    // SAFETY: AVMediaTypeAudio is an immutable AVFoundation framework constant with
    // process lifetime. `av-foundation` exposes framework statics through an extern
    // declaration, so reading the reference requires an unsafe block.
    let audio_media_type = unsafe { AVMediaTypeAudio };
    let status = AVCaptureDevice::authorization_status_for_media_type(audio_media_type);

    match status {
        AVAuthorizationStatusNotDetermined => MicrophonePermissionState::NotRequested,
        AVAuthorizationStatusAuthorized => MicrophonePermissionState::Granted,
        AVAuthorizationStatusDenied | AVAuthorizationStatusRestricted => {
            MicrophonePermissionState::Denied
        }
        _ => MicrophonePermissionState::Unavailable,
    }
}

pub fn microphone_permission_state() -> MicrophonePermissionState {
    #[cfg(target_os = "macos")]
    {
        return macos_microphone_permission_state();
    }

    #[cfg(not(target_os = "macos"))]
    {
        MicrophonePermissionState::Unavailable
    }
}

#[cfg(target_os = "macos")]
pub async fn request_microphone_permission() -> Result<MicrophonePermissionState, String> {
    use av_foundation::capture_device::AVCaptureDevice;
    use av_foundation::media_format::AVMediaTypeAudio;
    use parking_lot::Mutex;
    use std::sync::Arc;
    use tokio::sync::oneshot;

    match microphone_permission_state() {
        MicrophonePermissionState::Granted => return Ok(MicrophonePermissionState::Granted),
        MicrophonePermissionState::Denied => return Ok(MicrophonePermissionState::Denied),
        MicrophonePermissionState::Unavailable => {
            return Ok(MicrophonePermissionState::Unavailable)
        }
        MicrophonePermissionState::NotRequested => {}
    }

    let (sender, receiver) = oneshot::channel();
    let sender = Arc::new(Mutex::new(Some(sender)));
    let sender_for_callback = sender.clone();

    // SAFETY: AVMediaTypeAudio is an immutable AVFoundation framework constant with
    // process lifetime. The completion callback owns only an Arc to a one-shot sender.
    let audio_media_type = unsafe { AVMediaTypeAudio };
    AVCaptureDevice::request_access_for_media_type(audio_media_type, move |granted| {
        if let Some(sender) = sender_for_callback.lock().take() {
            let _ = sender.send(granted.as_bool());
        }
    });

    receiver
        .await
        .map(|granted| {
            if granted {
                MicrophonePermissionState::Granted
            } else {
                MicrophonePermissionState::Denied
            }
        })
        .map_err(|_| "macOS microphone permission callback was cancelled".to_string())
}

#[cfg(not(target_os = "macos"))]
pub async fn request_microphone_permission() -> Result<MicrophonePermissionState, String> {
    Ok(MicrophonePermissionState::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_state_serialization_is_stable() {
        assert_eq!(
            serde_json::to_string(&MicrophonePermissionState::NotRequested).unwrap(),
            r#""not_requested""#
        );
        assert_eq!(
            serde_json::to_string(&MicrophonePermissionState::Granted).unwrap(),
            r#""granted""#
        );
        assert_eq!(
            serde_json::to_string(&MicrophonePermissionState::Denied).unwrap(),
            r#""denied""#
        );
        assert_eq!(
            serde_json::to_string(&MicrophonePermissionState::Unavailable).unwrap(),
            r#""unavailable""#
        );
    }

    #[test]
    fn only_granted_state_reports_granted() {
        assert!(MicrophonePermissionState::Granted.is_granted());
        assert!(!MicrophonePermissionState::NotRequested.is_granted());
        assert!(!MicrophonePermissionState::Denied.is_granted());
        assert!(!MicrophonePermissionState::Unavailable.is_granted());
    }
}
