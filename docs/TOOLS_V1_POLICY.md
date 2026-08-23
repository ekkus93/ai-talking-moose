# Talking Moose V1 safe-tool policy

This document defines the V1 provider-neutral tool contract and the local enforcement boundary.

## Provider-neutral declaration contract

Every V1 tool has one Rust `ToolDeclaration` containing:

- a stable tool name and description;
- a JSON argument schema;
- a permission class;
- a privacy gate;
- a confirmation policy;
- a timeout;
- a maximum serialized input size; and
- a maximum serialized output size.

Gemini receives only the provider-facing function name, description, and JSON schema. Local permission,
privacy, confirmation, timeout, and size metadata are never delegated to the provider.

## Router enforcement

The Rust `ToolRouter` is authoritative. A model tool call must pass, in order:

1. exact registered-name lookup;
2. hard serialized input-size bound;
3. local permission/privacy/confirmation policy;
4. declared JSON-schema validation;
5. a hard timeout; and
6. a hard serialized output-size bound.

Failures return a structured `ToolErrorKind`; raw arguments, raw results, and backend error strings are not
included in normal logs or audit records. Repository-wide hard ceilings prevent a declaration from raising
its own timeout or size limits beyond the V1 maximums.

## V1 capability allowlist

The complete provider-visible V1 tool surface is:

| Tool | Class | Local authorization |
| --- | --- | --- |
| `get_current_time` | safe read-only | always locally available |
| `get_battery_level` | safe read-only | typed observer availability still applies |
| `get_active_application` | safe read-only | `active_app_observation` must be enabled before the OS query |
| `remember_fact` | memory mutation | default-off `memory_enabled` setting is the user's standing opt-in |

No generic filesystem, network/HTTP, shell, AppleScript, command, or process-execution tool exists in V1.
There is no provider-visible `CharacterAction` tool in V1. The router requires that any future declaration
of that class use per-invocation confirmation and that the invocation carry explicit local user confirmation,
so model origin can never become sufficient authorization by itself.

## Audit and retention

The router keeps at most 128 in-memory audit records. Each record contains only:

- the registered tool name (or the fixed label `unregistered`);
- timestamp;
- elapsed milliseconds;
- declared permission class;
- local permission outcome; and
- structured result category.

Raw arguments and raw results are never stored. Unknown model-supplied tool names are not retained because
an attacker could encode private content in the name itself. Audit records are process-local and are not
persisted to SQLite in V1.

## Regression policy

Tests lock the provider-visible tool names to the exact V1 allowlist and explicitly probe representative
forbidden names such as shell, AppleScript, file, HTTP, and process execution. Any expansion of the tool
surface therefore requires an intentional policy/test change rather than silently becoming provider-visible.
