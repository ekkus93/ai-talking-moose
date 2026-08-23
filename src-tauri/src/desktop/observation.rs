use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObserverStatus {
    Available,
    Denied,
    Unavailable,
    Unsupported,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObserverKind {
    IdleTime,
    SleepWake,
    Battery,
    ActiveApplication,
    WindowTitle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObserverErrorCode {
    InvalidValue,
    NoPowerSource,
    NoFrontmostApplication,
    PlatformApiFailure,
    RegistrationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverDiagnostic {
    pub kind: ObserverKind,
    pub status: ObserverStatus,
    pub error_code: Option<ObserverErrorCode>,
}

#[derive(Debug)]
pub enum ObserverResult<T> {
    Available(T),
    Denied,
    Unavailable(ObserverErrorCode),
    Unsupported,
    Error(ObserverErrorCode),
}

impl<T> ObserverResult<T> {
    pub fn status(&self) -> ObserverStatus {
        match self {
            Self::Available(_) => ObserverStatus::Available,
            Self::Denied => ObserverStatus::Denied,
            Self::Unavailable(_) => ObserverStatus::Unavailable,
            Self::Unsupported => ObserverStatus::Unsupported,
            Self::Error(_) => ObserverStatus::Error,
        }
    }

    pub fn diagnostic(&self, kind: ObserverKind) -> ObserverDiagnostic {
        let error_code = match self {
            Self::Unavailable(code) | Self::Error(code) => Some(*code),
            Self::Available(_) | Self::Denied | Self::Unsupported => None,
        };
        ObserverDiagnostic {
            kind,
            status: self.status(),
            error_code,
        }
    }

    pub fn into_available(self) -> Option<T> {
        match self {
            Self::Available(value) => Some(value),
            Self::Denied | Self::Unavailable(_) | Self::Unsupported | Self::Error(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdleObservation {
    pub seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryObservation {
    pub level_percent: u8,
    pub is_charging: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveApplicationObservation {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerEvent {
    Sleep,
    Wake,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_never_serialize_available_private_values() {
        let result = ObserverResult::Available(ActiveApplicationObservation {
            name: "Secret Project Editor".to_string(),
        });
        let diagnostic = result.diagnostic(ObserverKind::ActiveApplication);
        let json = serde_json::to_string(&diagnostic).unwrap();

        assert_eq!(diagnostic.status, ObserverStatus::Available);
        assert!(!json.contains("Secret Project Editor"));
        assert!(!json.contains("value"));
    }

    #[test]
    fn every_non_available_status_fails_closed() {
        let results: [ObserverResult<u8>; 4] = [
            ObserverResult::Denied,
            ObserverResult::Unavailable(ObserverErrorCode::NoPowerSource),
            ObserverResult::Unsupported,
            ObserverResult::Error(ObserverErrorCode::PlatformApiFailure),
        ];
        assert!(results
            .into_iter()
            .all(|result| result.into_available().is_none()));
    }
}
