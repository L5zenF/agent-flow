use std::collections::HashSet;

use super::{
    InfrastructureAclError, RawGatewayConfig, RawHeaderRule, RawRuleScope, util::unique_ids,
};

pub(super) fn validate_route_and_header_rule_subset(
    raw: &RawGatewayConfig,
) -> Result<(), InfrastructureAclError> {
    RouteSubsetAcl::new(raw)?.validate()
}

struct RouteSubsetAcl<'a> {
    raw: &'a RawGatewayConfig,
    provider_ids: HashSet<&'a str>,
    model_ids: HashSet<&'a str>,
    route_ids: HashSet<&'a str>,
}

impl<'a> RouteSubsetAcl<'a> {
    fn new(raw: &'a RawGatewayConfig) -> Result<Self, InfrastructureAclError> {
        let provider_ids = raw
            .providers
            .iter()
            .map(|provider| provider.id.as_str())
            .collect::<HashSet<_>>();
        let model_ids = raw
            .models
            .iter()
            .map(|model| model.id.as_str())
            .collect::<HashSet<_>>();
        let route_ids = unique_ids(raw.routes.iter().map(|route| route.id.as_str()), "route")?;
        unique_ids(
            raw.header_rules.iter().map(|rule| rule.id.as_str()),
            "header_rule",
        )?;

        Ok(Self {
            raw,
            provider_ids,
            model_ids,
            route_ids,
        })
    }

    fn validate(&self) -> Result<(), InfrastructureAclError> {
        self.validate_routes()?;
        self.validate_header_rules()?;
        Ok(())
    }

    fn validate_routes(&self) -> Result<(), InfrastructureAclError> {
        for route in &self.raw.routes {
            if route.matcher.trim().is_empty() {
                return Err(InfrastructureAclError::Validation(format!(
                    "route '{}' matcher cannot be empty",
                    route.id
                )));
            }
            if !self.provider_ids.contains(route.provider_id.as_str()) {
                return Err(InfrastructureAclError::Validation(format!(
                    "route '{}' references missing provider '{}'",
                    route.id, route.provider_id
                )));
            }
            if let Some(model_id) = route.model_id.as_deref()
                && !self.model_ids.contains(model_id)
            {
                return Err(InfrastructureAclError::Validation(format!(
                    "route '{}' references missing model '{}'",
                    route.id, model_id
                )));
            }
        }
        Ok(())
    }

    fn validate_header_rules(&self) -> Result<(), InfrastructureAclError> {
        for rule in &self.raw.header_rules {
            match rule.scope {
                RawRuleScope::Global => {
                    if rule.target_id.is_some() {
                        return Err(InfrastructureAclError::Validation(format!(
                            "header_rule '{}' must not define target_id for global scope",
                            rule.id
                        )));
                    }
                }
                RawRuleScope::Provider => {
                    self.validate_header_rule_target(rule, &self.provider_ids, "provider")?;
                }
                RawRuleScope::Model => {
                    self.validate_header_rule_target(rule, &self.model_ids, "model")?;
                }
                RawRuleScope::Route => {
                    self.validate_header_rule_target(rule, &self.route_ids, "route")?;
                }
            }

            if rule.actions.is_empty() {
                return Err(InfrastructureAclError::Validation(format!(
                    "header_rule '{}' must contain at least one action",
                    rule.id
                )));
            }
        }

        Ok(())
    }

    fn validate_header_rule_target(
        &self,
        rule: &RawHeaderRule,
        ids: &HashSet<&str>,
        kind: &str,
    ) -> Result<(), InfrastructureAclError> {
        let Some(target_id) = rule.target_id.as_deref() else {
            return Err(InfrastructureAclError::Validation(format!(
                "header_rule '{}' requires target_id for {kind} scope",
                rule.id
            )));
        };

        if !ids.contains(target_id) {
            return Err(InfrastructureAclError::Validation(format!(
                "header_rule '{}' references missing {kind} '{}'",
                rule.id, target_id
            )));
        }

        Ok(())
    }
}
