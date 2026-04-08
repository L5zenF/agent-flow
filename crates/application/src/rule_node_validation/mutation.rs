use crate::ApplicationError;

pub fn validate_value_node(node_id: &str, value: Option<&str>) -> Result<(), ApplicationError> {
    let Some(value) = value else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing value config"
        )));
    };
    if value.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' value cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_set_context_node(
    node_id: &str,
    key: Option<&str>,
    value_template: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(key) = key else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing set_context config"
        )));
    };
    let Some(value_template) = value_template else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing set_context config"
        )));
    };
    if key.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' context key cannot be empty"
        )));
    }
    if value_template.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' context value_template cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_log_node(node_id: &str, message: Option<&str>) -> Result<(), ApplicationError> {
    let Some(message) = message else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing log config"
        )));
    };
    if message.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' log message cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_header_mutation_node(
    node_id: &str,
    name: Option<&str>,
    value: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(name) = name else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing header config"
        )));
    };
    let Some(value) = value else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing header config"
        )));
    };
    if name.trim().is_empty() || value.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' header name/value cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_header_name_node(
    node_id: &str,
    name: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(name) = name else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing remove_header config"
        )));
    };
    if name.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' header name cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_copy_header_node(
    node_id: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(from) = from else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing copy_header config"
        )));
    };
    let Some(to) = to else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing copy_header config"
        )));
    };
    if from.trim().is_empty() || to.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' copy header fields cannot be empty"
        )));
    }
    Ok(())
}
