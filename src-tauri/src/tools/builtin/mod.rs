use crate::app::state::AppSettings;
use crate::character::personality::CharacterConfig;
use crate::desktop::macos::SystemDesktopMonitor;
use crate::desktop::observation::{ObserverKind, ObserverResult};
use crate::memory::MemoryManager;
use crate::tools::policy::{ToolDeclaration, ToolPermissionLevel};
use chrono::Local;
use parking_lot::RwLock;
use serde_json::json;
use std::sync::Arc;

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
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
                permission: ToolPermissionLevel::SafeReadOnly,
            },
            ToolDeclaration {
                name: "get_battery_level".to_string(),
                description: "Get the current battery charge level and power status".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
                permission: ToolPermissionLevel::SafeReadOnly,
            },
            ToolDeclaration {
                name: "get_active_application".to_string(),
                description: "Get the name of the currently active frontmost application on the user's screen".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
                permission: ToolPermissionLevel::SafeReadOnly,
            },
            ToolDeclaration {
                name: "remember_fact".to_string(),
                description: "Remember an explicit fact about the user for future conversations".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "fact": {
                            "type": "string",
                            "description": "The fact to remember"
                        }
                    },
                    "required": ["fact"]
                }),
                permission: ToolPermissionLevel::MemoryMutation,
            },
        ]
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
            // Strict security: Reject unknown or arbitrary tool requests
            unknown => Err(format!("Tool '{}' is not registered or permitted", unknown)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::observation::{ObserverErrorCode, ObserverStatus};

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
}
