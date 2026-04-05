#[allow(warnings)]
mod bindings;

use bindings::exports::proxy_tools::proxy_node_plugin::node_plugin::{
    ContextEntry, ContextPatch, ContextPatchOp, ExecuteError, ExecuteInput, ExecuteOutput, Guest,
    LogEntry, LogLevel, NextPort,
};

struct Component;

impl Guest for Component {
    fn execute(input: ExecuteInput) -> Result<ExecuteOutput, ExecuteError> {
        let header_intent = input
            .request_headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case("x-intent"))
            .map(|header| header.value.as_str());
        let path = input.current_path.as_str();

        let intent = if matches!(header_intent, Some("chat")) || path.contains("/chat") {
            "chat"
        } else {
            "default"
        };

        Ok(ExecuteOutput {
            context_patch: Some(ContextPatch {
                ops: vec![ContextPatchOp::Set(ContextEntry {
                    key: "intent".to_string(),
                    value: intent.to_string(),
                })],
            }),
            header_ops: Vec::new(),
            path_rewrite: None,
            next_port: Some(NextPort {
                port: intent.to_string(),
            }),
            logs: vec![LogEntry {
                level: LogLevel::Info,
                message: format!(
                    "intent-classifier routed path '{}' to port '{}'",
                    input.current_path, intent
                ),
            }],
        })
    }
}

bindings::export!(Component with_types_in bindings);
