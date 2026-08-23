use crate::app::state::AppSettings;
use crate::character::personality::CharacterConfig;
use crate::desktop::macos::SystemDesktopMonitor;
use crate::desktop::observation::{ObserverKind, ObserverResult};
use crate::memory::MemoryManager;
use crate::tools::policy::{
    ToolConfirmationPolicy, ToolDeclaration, ToolExecutionPolicy, ToolPermissionLevel,
    ToolPrivacyGate,
};
use chrono::Local;
use parking_lot::RwLock;
use serde_json::json;
use std::sync::Arc;

pub const V1_TOOL_NAMES: &[&str] = &[
    "get_current_time",
    "get_battery_level",
    "get_active_application",
    "remember_fact",
];

pub struct BuiltinTools {
    pub memory_manager: Arc<MemoryManager>,
    pub character_config: CharacterConfig,
    pub settings: Arc<RwLock<AppSettings>>,
}

fn observer_unavailable<T>(kind: ObserverKind, result: &ObserverResult<T>) -> serde_json::Value {
    let diagnostic = result.diagnostic(kind);
    json!({
        "available": false,
        "observer": diagnostic.kind,
        "status": diagnostic.status,
        "error_code": diagnostic.error_code,
        "error": "Desktop observation is not available",
    })
}

impl BuiltinTools {
    pub fn get_declarations(&self) -> Vec<ToolDeclaration> {
        vec![
            ToolDeclaration {
                name: "get_current_time".to_string(),
                description: "Get current local time and date".to_string(),
                parameters: no_argument_schema(),
                permission: ToolPermissionLevel::SafeReadOnly,
                privacy_gate: ToolPrivacyGate::None,
                confirmation: ToolConfirmationPolicy::None,
                execution: ToolExecutionPolicy::new(250, 64, 1_024),
            },
            ToolDeclaration {
                name: "get_battery_level".to_string(),
                description: "Get the current battery charge level and power status".to_string(),
                parameters: no_argument_schema(),
                permission: ToolPermissionLevel::SafeReadOnly,
                privacy_gate: ToolPrivacyGate::None,
                confirmation: ToolConfirmationPolicy::None,
                execution: ToolExecutionPolicy::new(500, 64, 1_024),
            },
            ToolDeclaration {
                name: "get_active_application".to_string(),
                description: "Get the name of the currently active frontmost application on the user's screen".to_string(),
                parameters: no_argument_schema(),
                permission: ToolPermissionLevel::SafeReadOnly,
                privacy_gate: ToolPrivacyGate::ActiveApplication,
                confirmation: ToolConfirmationPolicy::None,
                execution: ToolExecutionPolicy::new(500, 64, 2_048),
            },
            ToolDeclaration {
                name: "remember_fact".to_string(),
                description: "Remember an explicit fact about the user for future conversations".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "fact": {
                            "type": "string",
                            "description": "The explicit fact to remember",
                            "minLength": 1,
                            "maxLength": 1024
                        }
                    },
                    "required": ["fact"],
                    "additionalProperties": false
                }),
                permission: ToolPermissionLevel::MemoryMutation,
                privacy_gate: ToolPrivacyGate::Memory,
                confirmation: ToolConfirmationPolicy::SettingOptIn,
                execution: ToolExecutionPolicy::new(1_500, 4_096, 1_024),
            },
        ]
    }

    pub(crate) fn privacy_gate_allowed(&self, gate: ToolPrivacyGate) -> bool {
        match gate {
            ToolPrivacyGate::None => true,
            ToolPrivacyGate::ActiveApplication => self.settings.read().active_app_observation,
            ToolPrivacyGate::Memory => self.settings.read().memory_enabled,
        }
    }

    pub async fn execute(
        &self,
        name: &str,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match name {
            "get_current_time" => {
                let now = Local::now();
                Ok(json!({
                    "time": now.format("%I:%M %p").to_string(),
                    "date": now.format("%A, %B %d, %Y").to_string()
                }))
            }
            "get_battery_level" => {
                let result = SystemDesktopMonitor::get_battery_state();
                match result {
                    ObserverResult::Available(observation) => Ok(json!({
                        "available": true,
                        "level_percentage": observation.level_percent,
                        "is_charging": observation.is_charging
                    })),
                    other => Ok(observer_unavailable(ObserverKind::Battery, &other)),
                }
            }
            "get_active_application" => {
                let allowed = self.settings.read().active_app_observation;
                let result = SystemDesktopMonitor::get_active_application(allowed);
                match result {
                    ObserverResult::Available(observation) => Ok(json!({
                        "available": true,
                        "active_application": observation.name
                    })),
                    other => Ok(observer_unavailable(
                        ObserverKind::ActiveApplication,
                        &other,
                    )),
                }
            }
            "remember_fact" => {
                if !self.settings.read().memory_enabled {
                    return Err("Memory is disabled by user settings".to_string());
                }
                let fact = args["fact"]
                    .as_str()
                    .ok_or_else(|| "Missing required parameter 'fact'".to_string())?;
                let id = self.memory_manager.remember(fact, Some("conversation"))?;
                Ok(json!({ "status": "remembered", "memory_id": id }))
            }
            // Defense in depth: direct callers cannot use this executor as an escape hatch.
            _ => Err("Tool is not registered or permitted".to_string()),
        }
    }
}

fn no_argument_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::observation::{ObserverErrorCode, ObserverStatus};
    use crate::persistence::sqlite::Database;

    fn tools() -> BuiltinTools {
        let db = Arc::new(Database::new_in_memory().unwrap());
        BuiltinTools {
            memory_manager: Arc::new(MemoryManager::new(db)),
            character_config: CharacterConfig::default(),
            settings: Arc::new(RwLock::new(AppSettings::default())),
        }
    }

    #[test]
    fn observer_tool_failures_expose_only_safe_status_metadata() {
        let response = observer_unavailable::<String>(
            ObserverKind::ActiveApplication,
            &ObserverResult::Error(ObserverErrorCode::PlatformApiFailure),
        );
        assert_eq!(response["available"], false);
        assert_eq!(response["status"], "error");
        assert_eq!(response["error_code"], "platform_api_failure");
        assert!(response.get("active_application").is_none());

        let denied = ObserverResult::<String>::Denied;
        assert_eq!(
            denied.diagnostic(ObserverKind::ActiveApplication).status,
            ObserverStatus::Denied
        );
    }

    #[test]
    fn v1_tool_surface_is_an_exact_allowlist_without_generic_execution() {
        let declarations = tools().get_declarations();
        let names: Vec<&str> = declarations.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(names.as_slice(), V1_TOOL_NAMES);

        for forbidden in [
            "shell",
            "applescript",
            "filesystem",
            "http_request",
            "spawn_process",
            "execute_command",
        ] {
            assert!(
                declarations.iter().all(|tool| {
                    !tool.name.to_ascii_lowercase().contains(forbidden)
                        && !tool.description.to_ascii_lowercase().contains(forbidden)
                }),
                "prohibited generic capability appeared in the V1 tool surface: {forbidden}"
            );
        }
    }

    #[test]
    fn declaration_permissions_match_v1_safety_policy() {
        let declarations = tools().get_declarations();
        for tool in declarations {
            match tool.name.as_str() {
                "get_current_time" | "get_battery_level" => {
                    assert_eq!(tool.permission, ToolPermissionLevel::SafeReadOnly);
                    assert_eq!(tool.privacy_gate, ToolPrivacyGate::None);
                    assert_eq!(tool.confirmation, ToolConfirmationPolicy::None);
                }
                "get_active_application" => {
                    assert_eq!(tool.permission, ToolPermissionLevel::SafeReadOnly);
                    assert_eq!(tool.privacy_gate, ToolPrivacyGate::ActiveApplication);
                    assert_eq!(tool.confirmation, ToolConfirmationPolicy::None);
                }
                "remember_fact" => {
                    assert_eq!(tool.permission, ToolPermissionLevel::MemoryMutation);
                    assert_eq!(tool.privacy_gate, ToolPrivacyGate::Memory);
                    assert_eq!(tool.confirmation, ToolConfirmationPolicy::SettingOptIn);
                }
                other => panic!("unexpected V1 tool declaration: {other}"),
            }
        }
    }
}
