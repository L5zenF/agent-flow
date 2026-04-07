use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

use domain::{GatewayCatalog, GatewayConfigSource, WorkflowIndex};

use crate::config_acl::{InfrastructureAclError, map_gateway_config, parse_raw_gateway_config};

#[derive(Debug)]
pub enum InfrastructureStoreError {
    Io(std::io::Error),
    Acl(InfrastructureAclError),
}

impl Display for InfrastructureStoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::Acl(error) => error.fmt(formatter),
        }
    }
}

impl Error for InfrastructureStoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Acl(error) => Some(error),
        }
    }
}

impl From<std::io::Error> for InfrastructureStoreError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<InfrastructureAclError> for InfrastructureStoreError {
    fn from(value: InfrastructureAclError) -> Self {
        Self::Acl(value)
    }
}

pub fn load_gateway_config_from_path(
    path: &Path,
) -> Result<(GatewayCatalog, WorkflowIndex), InfrastructureStoreError> {
    let raw = std::fs::read_to_string(path)?;
    let parsed = parse_raw_gateway_config(&raw)?;
    Ok(map_gateway_config(&parsed)?)
}

#[derive(Debug, Clone)]
pub struct GatewayConfigFileSource {
    path: PathBuf,
}

impl GatewayConfigFileSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl GatewayConfigSource for GatewayConfigFileSource {
    type Error = InfrastructureStoreError;

    fn load_gateway_state(&self) -> Result<(GatewayCatalog, WorkflowIndex), Self::Error> {
        load_gateway_config_from_path(&self.path)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{InfrastructureStoreError, load_gateway_config_from_path};

    fn temp_file_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        path.push(format!("proxy-tools-infra-store-{name}-{stamp}.toml"));
        path
    }

    #[test]
    fn loads_and_maps_gateway_config_from_file() {
        let config_path = temp_file_path("loads");
        std::fs::write(
            &config_path,
            r#"
active_workflow_id = "chat-routing"

[[providers]]
id = "openai"
name = "OpenAI"

[[models]]
id = "gpt-4o"
name = "GPT-4o"
provider_id = "openai"

[[workflows]]
id = "chat-routing"
file = "chat-routing.toml"
"#,
        )
        .expect("test config file should be writable");

        let (catalog, workflows) =
            load_gateway_config_from_path(&config_path).expect("file should parse and map");

        assert_eq!(catalog.providers().len(), 1);
        assert_eq!(catalog.models().iter().count(), 1);
        assert_eq!(workflows.iter().count(), 1);
        assert_eq!(
            workflows
                .active()
                .expect("active workflow should exist")
                .id()
                .as_str(),
            "chat-routing"
        );

        let _ = std::fs::remove_file(config_path);
    }

    #[test]
    fn reports_io_error_for_missing_file() {
        let config_path = temp_file_path("missing");
        let error = load_gateway_config_from_path(&config_path)
            .expect_err("missing file should return io error");

        assert!(matches!(error, InfrastructureStoreError::Io(_)));
    }
}
