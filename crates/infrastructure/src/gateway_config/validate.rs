mod graph;
mod ids;
mod mapping;

use std::collections::HashSet;

use application::{GatewayValidationInput, validate_gateway_basics};

use crate::gateway_config::legacy::normalize_legacy_rule_graph;
use crate::gateway_config::model::GatewayConfig;

pub(crate) use graph::validate_rule_graph;
pub(crate) use ids::unique_ids;
use mapping::map_gateway_validation_input;

pub fn parse_config(raw: &str) -> Result<GatewayConfig, Box<dyn std::error::Error>> {
    let config: GatewayConfig = toml::from_str(raw)?;
    let config = normalize_legacy_rule_graph(config);
    validate_config(&config)?;
    Ok(config)
}

pub fn validate_config(config: &GatewayConfig) -> Result<(), Box<dyn std::error::Error>> {
    let validation_input: GatewayValidationInput = map_gateway_validation_input(config);
    let basic_validation =
        validate_gateway_basics(&validation_input).map_err(|error| error.to_string())?;
    let provider_ids = basic_validation
        .provider_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let model_ids = basic_validation
        .model_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();

    if let Some(graph) = &config.rule_graph {
        validate_rule_graph(graph, &provider_ids, &model_ids, &config.models)?;
    }

    Ok(())
}
