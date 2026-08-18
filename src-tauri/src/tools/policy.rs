use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermissionLevel {
    SafeReadOnly,
    CharacterAction,
    MemoryMutation,
    Denied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub permission: ToolPermissionLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAuditRecord {
    pub tool_name: String,
    pub timestamp: String,
    pub success: bool,
}
