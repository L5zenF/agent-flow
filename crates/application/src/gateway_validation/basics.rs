use std::collections::HashSet;

use crate::{
    ApplicationError, GatewayValidationInput, GatewayValidationResult, HeaderRuleValidationInput,
    RuleScopeInput,
};

use super::shared::unique_ids;
use super::workflow_index::WorkflowIndexValidator;

pub fn validate_gateway_basics(
    input: &GatewayValidationInput,
) -> Result<GatewayValidationResult, ApplicationError> {
    GatewayBasicsValidator::new(input)?.validate()
}

struct GatewayBasicsValidator<'a> {
    input: &'a GatewayValidationInput,
    provider_ids: HashSet<String>,
    model_ids: HashSet<String>,
    route_ids: HashSet<String>,
}

impl<'a> GatewayBasicsValidator<'a> {
    fn new(input: &'a GatewayValidationInput) -> Result<Self, ApplicationError> {
        let provider_ids = unique_ids(
            input.providers.iter().map(|provider| provider.id.as_str()),
            "provider",
        )?;
        let model_ids = unique_ids(input.models.iter().map(|model| model.id.as_str()), "model")?;
        let route_ids = unique_ids(input.routes.iter().map(|route| route.id.as_str()), "route")?;
        unique_ids(
            input.header_rules.iter().map(|rule| rule.id.as_str()),
            "header_rule",
        )?;

        Ok(Self {
            input,
            provider_ids,
            model_ids,
            route_ids,
        })
    }

    fn validate(self) -> Result<GatewayValidationResult, ApplicationError> {
        self.validate_providers()?;
        self.validate_models()?;
        self.validate_routes()?;
        self.validate_header_rules()?;
        WorkflowIndexValidator::new(
            self.input.workflows_dir.as_deref(),
            self.input.active_workflow_id.as_deref(),
            &self.input.workflows,
        )?
        .validate()?;

        Ok(GatewayValidationResult {
            provider_ids: self.provider_ids,
            model_ids: self.model_ids,
            route_ids: self.route_ids,
        })
    }

    fn validate_providers(&self) -> Result<(), ApplicationError> {
        for provider in &self.input.providers {
            if provider.id.trim().is_empty() {
                return Err(ApplicationError::Validation(
                    "provider id cannot be empty".to_string(),
                ));
            }
            if provider.base_url.trim().is_empty() {
                return Err(ApplicationError::Validation(format!(
                    "provider '{}' base_url cannot be empty",
                    provider.id
                )));
            }
        }
        Ok(())
    }

    fn validate_models(&self) -> Result<(), ApplicationError> {
        for model in &self.input.models {
            if !self.provider_ids.contains(model.provider_id.as_str()) {
                return Err(ApplicationError::Validation(format!(
                    "model '{}' references missing provider '{}'",
                    model.id, model.provider_id
                )));
            }
        }
        Ok(())
    }

    fn validate_routes(&self) -> Result<(), ApplicationError> {
        for route in &self.input.routes {
            if route.matcher.trim().is_empty() {
                return Err(ApplicationError::Validation(format!(
                    "route '{}' matcher cannot be empty",
                    route.id
                )));
            }
            if !self.provider_ids.contains(route.provider_id.as_str()) {
                return Err(ApplicationError::Validation(format!(
                    "route '{}' references missing provider '{}'",
                    route.id, route.provider_id
                )));
            }
            if let Some(model_id) = route.model_id.as_deref()
                && !self.model_ids.contains(model_id)
            {
                return Err(ApplicationError::Validation(format!(
                    "route '{}' references missing model '{}'",
                    route.id, model_id
                )));
            }
        }
        Ok(())
    }

    fn validate_header_rules(&self) -> Result<(), ApplicationError> {
        for rule in &self.input.header_rules {
            match rule.scope {
                RuleScopeInput::Global => {
                    if rule.target_id.is_some() {
                        return Err(ApplicationError::Validation(format!(
                            "header_rule '{}' must not define target_id for global scope",
                            rule.id
                        )));
                    }
                }
                RuleScopeInput::Provider => {
                    self.validate_rule_target(rule, &self.provider_ids, "provider")?;
                }
                RuleScopeInput::Model => {
                    self.validate_rule_target(rule, &self.model_ids, "model")?;
                }
                RuleScopeInput::Route => {
                    self.validate_rule_target(rule, &self.route_ids, "route")?;
                }
            }

            if rule.actions_len == 0 {
                return Err(ApplicationError::Validation(format!(
                    "header_rule '{}' must contain at least one action",
                    rule.id
                )));
            }
        }
        Ok(())
    }

    fn validate_rule_target(
        &self,
        rule: &HeaderRuleValidationInput,
        ids: &HashSet<String>,
        kind: &str,
    ) -> Result<(), ApplicationError> {
        let Some(target_id) = rule.target_id.as_deref() else {
            return Err(ApplicationError::Validation(format!(
                "header_rule '{}' requires target_id for {kind} scope",
                rule.id
            )));
        };

        if !ids.contains(target_id) {
            return Err(ApplicationError::Validation(format!(
                "header_rule '{}' references missing {kind} '{}'",
                rule.id, target_id
            )));
        }

        Ok(())
    }
}
