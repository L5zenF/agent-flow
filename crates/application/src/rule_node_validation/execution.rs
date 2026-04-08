use std::collections::HashSet;
use std::path::{Component, Path};

use crate::ApplicationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WasmCapabilityInput {
    Log,
    Fs,
    Network,
}

pub fn validate_code_runner_node(
    node_id: &str,
    timeout_ms: Option<u64>,
    max_memory_bytes: Option<u64>,
    code: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(timeout_ms) = timeout_ms else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing code_runner config"
        )));
    };
    let Some(max_memory_bytes) = max_memory_bytes else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing code_runner config"
        )));
    };
    let Some(code) = code else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing code_runner config"
        )));
    };

    if timeout_ms == 0 {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' timeout_ms must be greater than zero"
        )));
    }
    if max_memory_bytes == 0 {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' max_memory_bytes must be greater than zero"
        )));
    }
    if code.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' code cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_wasm_plugin_node(
    node_id: &str,
    plugin_id: Option<&str>,
    timeout_ms: Option<u64>,
    fuel: Option<Option<u64>>,
    max_memory_bytes: Option<u64>,
    granted_capabilities: &[WasmCapabilityInput],
    read_dirs: &[String],
    write_dirs: &[String],
    allowed_hosts: &[String],
) -> Result<(), ApplicationError> {
    let Some(plugin_id) = plugin_id else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing wasm_plugin config"
        )));
    };
    let Some(timeout_ms) = timeout_ms else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing wasm_plugin config"
        )));
    };
    let Some(fuel) = fuel else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing wasm_plugin config"
        )));
    };
    let Some(max_memory_bytes) = max_memory_bytes else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing wasm_plugin config"
        )));
    };

    if plugin_id.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' plugin_id cannot be empty"
        )));
    }
    if timeout_ms == 0 {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' timeout_ms must be greater than zero"
        )));
    }
    if matches!(fuel, Some(0)) {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' fuel must be greater than zero when set"
        )));
    }
    if max_memory_bytes == 0 {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' max_memory_bytes must be greater than zero"
        )));
    }

    let grants = granted_capabilities.iter().copied().collect::<HashSet<_>>();
    let has_fs = grants.contains(&WasmCapabilityInput::Fs);
    let has_network = grants.contains(&WasmCapabilityInput::Network);

    if has_fs {
        if read_dirs.is_empty() && write_dirs.is_empty() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' fs capability requires read_dirs or write_dirs"
            )));
        }
    } else if !read_dirs.is_empty() || !write_dirs.is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' fs directories require an fs capability grant"
        )));
    }

    validate_wasm_paths(node_id, "read_dirs", read_dirs)?;
    validate_wasm_paths(node_id, "write_dirs", write_dirs)?;

    if has_network {
        if allowed_hosts.is_empty() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' network capability requires allowed_hosts"
            )));
        }
    } else if !allowed_hosts.is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' allowed_hosts require a network capability grant"
        )));
    }

    validate_wasm_hosts(node_id, allowed_hosts)?;
    Ok(())
}

fn validate_wasm_paths(
    node_id: &str,
    field: &str,
    paths: &[String],
) -> Result<(), ApplicationError> {
    for path in paths {
        if path.trim().is_empty() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' {field} cannot contain empty paths"
            )));
        }
        let path_ref = Path::new(path);
        if path_ref.is_absolute() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' {field} must use relative paths"
            )));
        }
        if path_ref.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        }) {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' {field} must not contain parent traversal"
            )));
        }
    }
    Ok(())
}

fn validate_wasm_hosts(node_id: &str, hosts: &[String]) -> Result<(), ApplicationError> {
    for host in hosts {
        if host.trim().is_empty() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' allowed_hosts cannot contain empty hosts"
            )));
        }
    }
    Ok(())
}
