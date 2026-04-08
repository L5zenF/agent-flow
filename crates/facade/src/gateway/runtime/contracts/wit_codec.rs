use infrastructure::plugin_runtime_contract::{
    RuntimeContextPatchOp, RuntimeExecuteInput, RuntimeExecuteOutput, RuntimeHeaderOp,
    RuntimeLogEntry,
};

use super::super::exports::proxy_tools::proxy_node_plugin::node_plugin::{
    CapabilityDeclaration, CapabilityGrant, ContextEntry, ContextPatchOp, ExecuteInput,
    ExecuteOutput, HeaderOp, JsonDocument, NodeConfig, PluginManifest, RequestHeader,
};
use super::{from_wit_log_level, to_wit_capability_kind};

pub(super) fn to_wit_execute_input(input: &RuntimeExecuteInput) -> ExecuteInput {
    ExecuteInput {
        request_method: input.request_method.clone(),
        current_path: input.current_path.clone(),
        request_headers: input
            .request_headers
            .iter()
            .map(|header| RequestHeader {
                name: header.name.clone(),
                value: header.value.clone(),
            })
            .collect(),
        workflow_context: input
            .workflow_context
            .iter()
            .map(|(key, value)| ContextEntry {
                key: key.clone(),
                value: value.clone(),
            })
            .collect(),
        selected_provider_id: input.selected_provider_id.clone(),
        selected_model_id: input.selected_model_id.clone(),
        node_config: NodeConfig {
            manifest: PluginManifest {
                id: input.node_config.manifest.id.clone(),
                name: input.node_config.manifest.name.clone(),
                version: input.node_config.manifest.version.clone(),
                description: input.node_config.manifest.description.clone(),
                supported_output_ports: input.node_config.manifest.supported_output_ports.clone(),
                default_config_schema_hints: input
                    .node_config
                    .manifest
                    .default_config_schema_hints_json
                    .as_ref()
                    .map(|json| JsonDocument { json: json.clone() }),
                capabilities: input
                    .node_config
                    .manifest
                    .capabilities
                    .iter()
                    .map(|capability| CapabilityDeclaration {
                        kind: to_wit_capability_kind(&capability.kind),
                        required: capability.required,
                        scope: capability.scope.clone(),
                        description: capability.description.clone(),
                    })
                    .collect(),
            },
            grants: input
                .node_config
                .grants
                .iter()
                .map(|grant| CapabilityGrant {
                    kind: to_wit_capability_kind(&grant.kind),
                    allowed: grant.allowed,
                    scope: grant.scope.clone(),
                })
                .collect(),
            config: input
                .node_config
                .config_json
                .as_ref()
                .map(|json| JsonDocument { json: json.clone() }),
        },
    }
}

pub(super) fn from_wit_execute_output(output: ExecuteOutput) -> RuntimeExecuteOutput {
    RuntimeExecuteOutput {
        context_ops: output
            .context_patch
            .map(|patch| {
                patch
                    .ops
                    .into_iter()
                    .map(|op| match op {
                        ContextPatchOp::Set(entry) => RuntimeContextPatchOp::Set {
                            key: entry.key,
                            value: entry.value,
                        },
                        ContextPatchOp::Remove(key) => RuntimeContextPatchOp::Remove { key },
                    })
                    .collect()
            })
            .unwrap_or_default(),
        header_ops: output
            .header_ops
            .into_iter()
            .map(|op| match op {
                HeaderOp::Set(header) => RuntimeHeaderOp::Set {
                    name: header.name,
                    value: header.value,
                },
                HeaderOp::Append(header) => RuntimeHeaderOp::Append {
                    name: header.name,
                    value: header.value,
                },
                HeaderOp::Remove(name) => RuntimeHeaderOp::Remove { name },
            })
            .collect(),
        path_rewrite: output.path_rewrite.map(|path| path.path),
        next_port: output.next_port.map(|port| port.port),
        logs: output
            .logs
            .into_iter()
            .map(|log| RuntimeLogEntry {
                level: from_wit_log_level(log.level),
                message: log.message,
            })
            .collect(),
    }
}
