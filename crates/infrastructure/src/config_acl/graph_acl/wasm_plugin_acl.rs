use std::collections::HashSet;
use std::path::{Component, Path};

use super::super::{InfrastructureAclError, RawWasmCapability, RawWasmPluginNodeConfig};

pub(super) struct WasmPluginAcl<'a> {
    node_id: &'a str,
    config: &'a RawWasmPluginNodeConfig,
}

impl<'a> WasmPluginAcl<'a> {
    pub(super) fn new(
        node_id: &'a str,
        config: Option<&'a RawWasmPluginNodeConfig>,
    ) -> Result<Self, InfrastructureAclError> {
        let Some(config) = config else {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{node_id}' missing wasm_plugin config"
            )));
        };
        Ok(Self { node_id, config })
    }

    pub(super) fn from_config(node_id: &'a str, config: &'a RawWasmPluginNodeConfig) -> Self {
        Self { node_id, config }
    }

    pub(super) fn validate(&self) -> Result<(), InfrastructureAclError> {
        self.validate_limits()?;
        self.validate_capability_scopes()?;
        self.validate_hosts()?;
        Ok(())
    }

    fn validate_limits(&self) -> Result<(), InfrastructureAclError> {
        if self.config.plugin_id.trim().is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{}' plugin_id cannot be empty",
                self.node_id
            )));
        }
        if self.config.timeout_ms == 0 {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{}' timeout_ms must be greater than zero",
                self.node_id
            )));
        }
        if matches!(self.config.fuel, Some(0)) {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{}' fuel must be greater than zero when set",
                self.node_id
            )));
        }
        if self.config.max_memory_bytes == 0 {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{}' max_memory_bytes must be greater than zero",
                self.node_id
            )));
        }
        Ok(())
    }

    fn validate_capability_scopes(&self) -> Result<(), InfrastructureAclError> {
        let grants = self
            .config
            .granted_capabilities
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        let has_fs = grants.contains(&RawWasmCapability::Fs);
        let has_network = grants.contains(&RawWasmCapability::Network);

        if has_fs {
            if self.config.read_dirs.is_empty() && self.config.write_dirs.is_empty() {
                return Err(InfrastructureAclError::Validation(format!(
                    "rule_graph node '{}' fs capability requires read_dirs or write_dirs",
                    self.node_id
                )));
            }
        } else if !self.config.read_dirs.is_empty() || !self.config.write_dirs.is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{}' fs directories require an fs capability grant",
                self.node_id
            )));
        }

        self.validate_paths("read_dirs", &self.config.read_dirs)?;
        self.validate_paths("write_dirs", &self.config.write_dirs)?;

        if has_network {
            if self.config.allowed_hosts.is_empty() {
                return Err(InfrastructureAclError::Validation(format!(
                    "rule_graph node '{}' network capability requires allowed_hosts",
                    self.node_id
                )));
            }
        } else if !self.config.allowed_hosts.is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{}' allowed_hosts require a network capability grant",
                self.node_id
            )));
        }

        Ok(())
    }

    fn validate_paths(&self, field: &str, paths: &[String]) -> Result<(), InfrastructureAclError> {
        if paths.is_empty() {
            return Ok(());
        }

        for path in paths {
            if path.trim().is_empty() {
                return Err(InfrastructureAclError::Validation(format!(
                    "rule_graph node '{}' {field} cannot contain empty paths",
                    self.node_id
                )));
            }
            let path_ref = Path::new(path);
            if path_ref.is_absolute() {
                return Err(InfrastructureAclError::Validation(format!(
                    "rule_graph node '{}' {field} must use relative paths",
                    self.node_id
                )));
            }
            if path_ref.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::Prefix(_) | Component::RootDir
                )
            }) {
                return Err(InfrastructureAclError::Validation(format!(
                    "rule_graph node '{}' {field} must not contain parent traversal",
                    self.node_id
                )));
            }
        }

        Ok(())
    }

    fn validate_hosts(&self) -> Result<(), InfrastructureAclError> {
        if self.config.allowed_hosts.is_empty() {
            return Ok(());
        }

        for host in &self.config.allowed_hosts {
            if host.trim().is_empty() {
                return Err(InfrastructureAclError::Validation(format!(
                    "rule_graph node '{}' allowed_hosts cannot contain empty hosts",
                    self.node_id
                )));
            }
        }

        Ok(())
    }
}
