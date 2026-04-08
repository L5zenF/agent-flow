#[cfg(test)]
mod tests {
    use crate::{
        ApplicationError, GatewayValidationInput, HeaderRuleValidationInput, ModelValidationInput,
        ProviderValidationInput, RouteValidationInput, RuleScopeInput, WorkflowValidationInput,
        validate_gateway_basics,
    };

    fn valid_input() -> GatewayValidationInput {
        GatewayValidationInput {
            workflows_dir: Some("workflows".to_string()),
            active_workflow_id: Some("default".to_string()),
            providers: vec![ProviderValidationInput {
                id: "kimi".to_string(),
                base_url: "https://api.kimi.com".to_string(),
            }],
            models: vec![ModelValidationInput {
                id: "kimi-k2".to_string(),
                provider_id: "kimi".to_string(),
            }],
            routes: vec![RouteValidationInput {
                id: "chat".to_string(),
                matcher: "method == \"POST\"".to_string(),
                provider_id: "kimi".to_string(),
                model_id: Some("kimi-k2".to_string()),
            }],
            header_rules: vec![HeaderRuleValidationInput {
                id: "global".to_string(),
                scope: RuleScopeInput::Global,
                target_id: None,
                actions_len: 1,
            }],
            workflows: vec![WorkflowValidationInput {
                id: "default".to_string(),
                file: "default.toml".to_string(),
            }],
        }
    }

    #[test]
    fn validates_gateway_basics_successfully() {
        validate_gateway_basics(&valid_input()).expect("validation should pass");
    }

    #[test]
    fn rejects_duplicate_workflow_files() {
        let mut input = valid_input();
        input.workflows.push(WorkflowValidationInput {
            id: "fallback".to_string(),
            file: "default.toml".to_string(),
        });
        let error = validate_gateway_basics(&input).expect_err("duplicate file should fail");
        assert!(matches!(error, ApplicationError::Validation(_)));
    }
}
