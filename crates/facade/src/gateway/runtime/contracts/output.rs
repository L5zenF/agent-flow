use std::collections::HashMap;

use infrastructure::plugin_registry::LoadedPlugin;
use infrastructure::plugin_runtime_contract::{
    RuntimeContextPatchOp, RuntimeExecuteOutput, RuntimeHeaderOp,
};

use super::log_output;

pub(super) fn apply_runtime_output(
    node_id: &str,
    plugin: &LoadedPlugin,
    output: &RuntimeExecuteOutput,
    workflow_context: &mut HashMap<String, String>,
    outgoing_headers: &mut HashMap<String, Vec<String>>,
    resolved_path: &mut String,
) {
    log_output(node_id, plugin, output);
    for op in &output.context_ops {
        match op {
            RuntimeContextPatchOp::Set { key, value } => {
                workflow_context.insert(key.clone(), value.clone());
            }
            RuntimeContextPatchOp::Remove { key } => {
                workflow_context.remove(key);
            }
        }
    }
    for op in &output.header_ops {
        match op {
            RuntimeHeaderOp::Set { name, value } => {
                outgoing_headers.insert(name.to_ascii_lowercase(), vec![value.clone()]);
            }
            RuntimeHeaderOp::Append { name, value } => {
                outgoing_headers
                    .entry(name.to_ascii_lowercase())
                    .or_default()
                    .push(value.clone());
            }
            RuntimeHeaderOp::Remove { name } => {
                outgoing_headers.remove(&name.to_ascii_lowercase());
            }
        }
    }
    if let Some(path_rewrite) = &output.path_rewrite {
        *resolved_path = path_rewrite.clone();
    }
}
