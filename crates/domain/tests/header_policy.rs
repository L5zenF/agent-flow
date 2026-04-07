use std::collections::HashMap;

use domain::{
    HeaderAction, HeaderPolicy, HeaderPolicyRequest, HeaderRule, RuleScope,
    evaluate_policy_expression, render_policy_template,
};

fn base_request<'a>(
    headers: &'a HashMap<String, String>,
    context: &'a HashMap<String, String>,
) -> HeaderPolicyRequest<'a> {
    HeaderPolicyRequest {
        method: "POST",
        path: "/v1/chat/completions",
        headers,
        context,
        provider_id: Some("openai"),
        provider_name: Some("OpenAI"),
        model_id: Some("gpt-4o"),
        route_id: Some("chat-route"),
    }
}

#[test]
fn resolves_policy_with_scope_ordering_and_actions() {
    let mut headers = HashMap::new();
    headers.insert("x-request-id".to_string(), "req-1".to_string());
    let mut context = HashMap::new();
    context.insert("tenant".to_string(), "acme".to_string());
    let request = base_request(&headers, &context);

    let policy = HeaderPolicy::new(vec![
        HeaderRule {
            id: "global".to_string(),
            enabled: true,
            scope: RuleScope::Global,
            target_id: None,
            when: None,
            actions: vec![HeaderAction::Set {
                name: "x-scope".to_string(),
                value: "global".to_string(),
            }],
        },
        HeaderRule {
            id: "provider".to_string(),
            enabled: true,
            scope: RuleScope::Provider,
            target_id: Some("openai".to_string()),
            when: None,
            actions: vec![HeaderAction::Set {
                name: "x-scope".to_string(),
                value: "provider".to_string(),
            }],
        },
        HeaderRule {
            id: "model".to_string(),
            enabled: true,
            scope: RuleScope::Model,
            target_id: Some("gpt-4o".to_string()),
            when: Some("method == \"POST\" && path.contains(\"/chat\")".to_string()),
            actions: vec![HeaderAction::Set {
                name: "x-model".to_string(),
                value: "${model.id}".to_string(),
            }],
        },
        HeaderRule {
            id: "route".to_string(),
            enabled: true,
            scope: RuleScope::Route,
            target_id: Some("chat-route".to_string()),
            when: None,
            actions: vec![HeaderAction::SetIfAbsent {
                name: "x-scope".to_string(),
                value: "route".to_string(),
            }],
        },
    ]);

    let resolved = policy
        .resolve(
            &[("authorization".to_string(), "Bearer token".to_string())],
            &request,
        )
        .expect("policy should resolve");
    let map = resolved.into_iter().collect::<HashMap<_, _>>();

    assert_eq!(map.get("x-scope"), Some(&"provider".to_string()));
    assert_eq!(map.get("x-model"), Some(&"gpt-4o".to_string()));
    assert_eq!(map.get("authorization"), Some(&"Bearer token".to_string()));
}

#[test]
fn evaluates_expressions_and_templates() {
    let mut headers = HashMap::new();
    headers.insert("x-tenant".to_string(), "acme".to_string());
    let mut context = HashMap::new();
    context.insert("trace_id".to_string(), "tr-1".to_string());
    let request = base_request(&headers, &context);

    let matched = evaluate_policy_expression(
        "method == \"POST\" && header[\"x-tenant\"] == \"acme\" && ctx.trace_id == \"tr-1\"",
        &request,
    )
    .expect("expression should evaluate");
    assert!(matched);

    let rendered = render_policy_template(
        "${provider.id}:${model.id}:${route.id}:${request.header.x-tenant}:${ctx.trace_id}",
        &request,
    )
    .expect("template should render");
    assert_eq!(rendered, "openai:gpt-4o:chat-route:acme:tr-1");
}
