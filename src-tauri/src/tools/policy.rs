use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermissionLevel {
    SafeReadOnly,
    CharacterAction,
    MemoryMutation,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPrivacyGate {
    None,
    ActiveApplication,
    Memory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolConfirmationPolicy {
    None,
    SettingOptIn,
    PerInvocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionPolicy {
    pub timeout_ms: u64,
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
}

impl ToolExecutionPolicy {
    pub const fn new(timeout_ms: u64, max_input_bytes: usize, max_output_bytes: usize) -> Self {
        Self {
            timeout_ms,
            max_input_bytes,
            max_output_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub permission: ToolPermissionLevel,
    pub privacy_gate: ToolPrivacyGate,
    pub confirmation: ToolConfirmationPolicy,
    pub execution: ToolExecutionPolicy,
}

impl ToolDeclaration {
    pub fn validate_arguments(&self, arguments: &Value) -> Result<(), ToolError> {
        validate_schema(&self.parameters, arguments)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermissionOutcome {
    NotEvaluated,
    Allowed,
    Denied,
    ConfirmationRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolErrorKind {
    NotFound,
    InputTooLarge,
    InvalidArguments,
    PermissionDenied,
    ConfirmationRequired,
    ConcurrencyLimit,
    Timeout,
    OutputTooLarge,
    ExecutionFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolError {
    pub kind: ToolErrorKind,
    pub message: String,
}

impl ToolError {
    pub fn from_kind(kind: ToolErrorKind) -> Self {
        let message = match kind {
            ToolErrorKind::NotFound => "Tool is not registered.",
            ToolErrorKind::InputTooLarge => "Tool input exceeds the allowed size.",
            ToolErrorKind::InvalidArguments => "Tool arguments do not match the declared schema.",
            ToolErrorKind::PermissionDenied => "Tool permission was denied by local policy.",
            ToolErrorKind::ConfirmationRequired => {
                "Tool requires explicit user confirmation before execution."
            }
            ToolErrorKind::ConcurrencyLimit => "Tool execution capacity is temporarily exhausted.",
            ToolErrorKind::Timeout => "Tool execution exceeded its time limit.",
            ToolErrorKind::OutputTooLarge => "Tool output exceeds the allowed size.",
            ToolErrorKind::ExecutionFailed => "Tool execution failed.",
        };
        Self {
            kind,
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ToolError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolResultCategory {
    Success,
    NotFound,
    InputTooLarge,
    InvalidArguments,
    PermissionDenied,
    ConfirmationRequired,
    ConcurrencyLimit,
    Timeout,
    OutputTooLarge,
    ExecutionFailed,
}

impl ToolResultCategory {
    pub fn from_result(result: &Result<Value, ToolError>) -> Self {
        match result {
            Ok(_) => Self::Success,
            Err(error) => match error.kind {
                ToolErrorKind::NotFound => Self::NotFound,
                ToolErrorKind::InputTooLarge => Self::InputTooLarge,
                ToolErrorKind::InvalidArguments => Self::InvalidArguments,
                ToolErrorKind::PermissionDenied => Self::PermissionDenied,
                ToolErrorKind::ConfirmationRequired => Self::ConfirmationRequired,
                ToolErrorKind::ConcurrencyLimit => Self::ConcurrencyLimit,
                ToolErrorKind::Timeout => Self::Timeout,
                ToolErrorKind::OutputTooLarge => Self::OutputTooLarge,
                ToolErrorKind::ExecutionFailed => Self::ExecutionFailed,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolAuditRecord {
    pub tool_name: String,
    pub timestamp: String,
    pub duration_ms: u64,
    pub permission: ToolPermissionLevel,
    pub permission_outcome: ToolPermissionOutcome,
    pub result_category: ToolResultCategory,
}

fn invalid_arguments() -> ToolError {
    ToolError::from_kind(ToolErrorKind::InvalidArguments)
}

fn validate_schema(schema: &Value, value: &Value) -> Result<(), ToolError> {
    if let Some(allowed) = schema.get("enum").and_then(Value::as_array) {
        if !allowed.contains(value) {
            return Err(invalid_arguments());
        }
    }

    let Some(schema_type) = schema.get("type").and_then(Value::as_str) else {
        return Err(invalid_arguments());
    };

    match schema_type {
        "object" => validate_object(schema, value),
        "string" => validate_string(schema, value),
        "integer" => {
            if value.as_i64().is_none() && value.as_u64().is_none() {
                return Err(invalid_arguments());
            }
            validate_number_bounds(schema, value)
        }
        "number" => {
            if !value.is_number() {
                return Err(invalid_arguments());
            }
            validate_number_bounds(schema, value)
        }
        "boolean" => value
            .is_boolean()
            .then_some(())
            .ok_or_else(invalid_arguments),
        "array" => validate_array(schema, value),
        _ => Err(invalid_arguments()),
    }
}

fn validate_object(schema: &Value, value: &Value) -> Result<(), ToolError> {
    let object = value.as_object().ok_or_else(invalid_arguments)?;
    let properties = schema.get("properties").and_then(Value::as_object);

    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for property in required {
            let name = property.as_str().ok_or_else(invalid_arguments)?;
            if !object.contains_key(name) {
                return Err(invalid_arguments());
            }
        }
    }

    if schema.get("additionalProperties").and_then(Value::as_bool) == Some(false) {
        for name in object.keys() {
            if !properties.is_some_and(|known| known.contains_key(name)) {
                return Err(invalid_arguments());
            }
        }
    }

    if let Some(properties) = properties {
        for (name, property_schema) in properties {
            if let Some(property_value) = object.get(name) {
                validate_schema(property_schema, property_value)?;
            }
        }
    }

    Ok(())
}

fn validate_string(schema: &Value, value: &Value) -> Result<(), ToolError> {
    let text = value.as_str().ok_or_else(invalid_arguments)?;
    let length = text.chars().count();
    if let Some(minimum) = schema.get("minLength").and_then(Value::as_u64) {
        if length < minimum as usize {
            return Err(invalid_arguments());
        }
    }
    if let Some(maximum) = schema.get("maxLength").and_then(Value::as_u64) {
        if length > maximum as usize {
            return Err(invalid_arguments());
        }
    }
    Ok(())
}

fn validate_number_bounds(schema: &Value, value: &Value) -> Result<(), ToolError> {
    let number = value.as_f64().ok_or_else(invalid_arguments)?;
    if let Some(minimum) = schema.get("minimum").and_then(Value::as_f64) {
        if number < minimum {
            return Err(invalid_arguments());
        }
    }
    if let Some(maximum) = schema.get("maximum").and_then(Value::as_f64) {
        if number > maximum {
            return Err(invalid_arguments());
        }
    }
    Ok(())
}

fn validate_array(schema: &Value, value: &Value) -> Result<(), ToolError> {
    let array = value.as_array().ok_or_else(invalid_arguments)?;
    if let Some(minimum) = schema.get("minItems").and_then(Value::as_u64) {
        if array.len() < minimum as usize {
            return Err(invalid_arguments());
        }
    }
    if let Some(maximum) = schema.get("maxItems").and_then(Value::as_u64) {
        if array.len() > maximum as usize {
            return Err(invalid_arguments());
        }
    }
    if let Some(item_schema) = schema.get("items") {
        for item in array {
            validate_schema(item_schema, item)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn declaration(schema: Value) -> ToolDeclaration {
        ToolDeclaration {
            name: "test_tool".to_string(),
            description: "Test tool".to_string(),
            parameters: schema,
            permission: ToolPermissionLevel::SafeReadOnly,
            privacy_gate: ToolPrivacyGate::None,
            confirmation: ToolConfirmationPolicy::None,
            execution: ToolExecutionPolicy::new(500, 1024, 1024),
        }
    }

    #[test]
    fn schema_validation_rejects_missing_extra_and_oversized_fields() {
        let tool = declaration(json!({
            "type": "object",
            "properties": {
                "fact": { "type": "string", "minLength": 1, "maxLength": 8 }
            },
            "required": ["fact"],
            "additionalProperties": false
        }));

        assert!(tool
            .validate_arguments(&json!({ "fact": "coffee" }))
            .is_ok());
        for invalid in [
            json!({}),
            json!({ "fact": "coffee", "extra": true }),
            json!({ "fact": "far too long" }),
            json!({ "fact": 7 }),
        ] {
            assert_eq!(
                tool.validate_arguments(&invalid).unwrap_err().kind,
                ToolErrorKind::InvalidArguments
            );
        }
    }

    #[test]
    fn schema_validation_supports_bounded_arrays_and_numbers() {
        let tool = declaration(json!({
            "type": "object",
            "properties": {
                "levels": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 2,
                    "items": { "type": "number", "minimum": 0, "maximum": 1 }
                }
            },
            "required": ["levels"],
            "additionalProperties": false
        }));

        assert!(tool
            .validate_arguments(&json!({ "levels": [0.25, 1] }))
            .is_ok());
        assert!(tool.validate_arguments(&json!({ "levels": [] })).is_err());
        assert!(tool
            .validate_arguments(&json!({ "levels": [0, 0.5, 1] }))
            .is_err());
        assert!(tool
            .validate_arguments(&json!({ "levels": [1.5] }))
            .is_err());
    }
}
