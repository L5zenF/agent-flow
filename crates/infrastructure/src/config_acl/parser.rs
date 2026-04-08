use super::{InfrastructureAclError, RawGatewayConfig};

pub fn parse_raw_gateway_config(raw: &str) -> Result<RawGatewayConfig, InfrastructureAclError> {
    toml::from_str(raw).map_err(|error| InfrastructureAclError::TomlDeserialize(error.to_string()))
}
