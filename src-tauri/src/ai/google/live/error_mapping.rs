fn provider_error_for_http_status(status: u16) -> ProviderError {
    let kind = match status {
        401 | 403 => ProviderErrorKind::Auth,
        404 => ProviderErrorKind::Model,
        429 => ProviderErrorKind::Quota,
        400 => ProviderErrorKind::Setup,
        _ if status >= 500 => ProviderErrorKind::Network,
        _ => ProviderErrorKind::Protocol,
    };
    ProviderError::from_kind(kind)
}

fn provider_error_for_connect(error: WebSocketError) -> ProviderError {
    match error {
        WebSocketError::Http(response) => {
            provider_error_for_http_status(response.status().as_u16())
        }
        _ => ProviderError::from_kind(ProviderErrorKind::Network),
    }
}

fn provider_error_from_server_error(error: &LiveServerError) -> ProviderError {
    let code = error.code;
    let status = error.status.as_deref().unwrap_or_default();
    let kind = if matches!(code, Some(401 | 403))
        || matches!(status, "UNAUTHENTICATED" | "PERMISSION_DENIED")
    {
        ProviderErrorKind::Auth
    } else if code == Some(429) || status == "RESOURCE_EXHAUSTED" {
        ProviderErrorKind::Quota
    } else if code == Some(404) || status == "NOT_FOUND" {
        ProviderErrorKind::Model
    } else if code == Some(400) || matches!(status, "INVALID_ARGUMENT" | "FAILED_PRECONDITION") {
        ProviderErrorKind::Setup
    } else {
        ProviderErrorKind::Protocol
    };
    ProviderError::from_kind(kind)
}

fn provider_error_for_close(code: Option<u16>) -> Option<ProviderError> {
    match code {
        None | Some(1000 | 1001) => None,
        Some(1008) => Some(ProviderError::from_kind(ProviderErrorKind::Auth)),
        Some(1011 | 1006) => Some(ProviderError::from_kind(ProviderErrorKind::Network)),
        Some(_) => Some(ProviderError::from_kind(ProviderErrorKind::Closed)),
    }
}

fn tool_declarations(config: &LiveSessionConfig) -> Option<Vec<serde_json::Value>> {
    if config.tools.is_empty() {
        return None;
    }
    let declarations = config
        .tools
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "parametersJsonSchema": tool.parameters,
            })
        })
        .collect::<Vec<_>>();
    Some(vec![json!({ "functionDeclarations": declarations })])
}
