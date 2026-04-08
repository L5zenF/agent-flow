use std::collections::HashSet;

use crate::ApplicationError;

pub fn validate_route_provider_node(
    node_id: &str,
    provider_id: Option<&str>,
    provider_ids: &HashSet<String>,
) -> Result<(), ApplicationError> {
    let Some(provider_id) = provider_id else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' missing route_provider config",
            node_id
        )));
    };
    if !provider_ids.contains(provider_id) {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' references missing provider '{}'",
            node_id, provider_id
        )));
    }
    Ok(())
}

pub fn validate_select_model_node(
    node_id: &str,
    provider_id: Option<&str>,
    model_id: Option<&str>,
    provider_ids: &HashSet<String>,
    model_ids: &HashSet<String>,
    model_provider_id: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(provider_id) = provider_id else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' missing select_model config",
            node_id
        )));
    };
    let Some(model_id) = model_id else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' missing select_model config",
            node_id
        )));
    };
    if !provider_ids.contains(provider_id) {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' references missing provider '{}'",
            node_id, provider_id
        )));
    }
    if !model_ids.contains(model_id) {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' references missing model '{}'",
            node_id, model_id
        )));
    }
    let Some(model_provider_id) = model_provider_id else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' references missing model '{}'",
            node_id, model_id
        )));
    };
    if model_provider_id != provider_id {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' model '{}' does not belong to provider '{}'",
            node_id, model_id, provider_id
        )));
    }
    Ok(())
}
