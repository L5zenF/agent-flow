use domain::{DomainError, ModelId, ProviderId, RouteId, WorkflowId};
use infrastructure::config_acl::{
    InfrastructureAclError, RawModel, RawProvider, RawWorkflowIndex, RawWorkflowIndexEntry,
    map_gateway_catalog, map_gateway_config, map_workflow_index, parse_raw_gateway_config,
};

#[test]
fn maps_gateway_catalog_from_raw_shapes() {
    let catalog = map_gateway_catalog(
        &[
            RawProvider {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
            },
            RawProvider {
                id: "anthropic".to_string(),
                name: "Anthropic".to_string(),
            },
        ],
        &[
            RawModel {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                provider_id: "openai".to_string(),
            },
            RawModel {
                id: "claude-sonnet".to_string(),
                name: "Claude Sonnet".to_string(),
                provider_id: "anthropic".to_string(),
            },
        ],
    )
    .expect("raw provider/model subsets should map into a valid gateway catalog");

    assert!(
        catalog
            .providers()
            .contains(&ProviderId::new("openai").expect("non-blank provider id"))
    );
    assert!(
        catalog
            .models()
            .get(&ModelId::new("gpt-4o").expect("non-blank model id"))
            .is_some()
    );
}

#[test]
fn rejects_unknown_provider_reference_in_gateway_mapping() {
    let error = map_gateway_catalog(
        &[RawProvider {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
        }],
        &[RawModel {
            id: "gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            provider_id: "missing".to_string(),
        }],
    )
    .expect_err("raw model must reference a known provider");

    assert_eq!(
        error,
        InfrastructureAclError::Domain(DomainError::UnknownProviderReference {
            model_id: ModelId::new("gpt-4o").expect("non-blank model id"),
            provider_id: ProviderId::new("missing").expect("non-blank provider id"),
        })
    );
}

#[test]
fn maps_workflow_index_with_explicit_active_workflow() {
    let workflow_index = map_workflow_index(&RawWorkflowIndex {
        workflows: vec![
            RawWorkflowIndexEntry {
                id: "primary".to_string(),
                name: None,
                file: None,
                description: None,
            },
            RawWorkflowIndexEntry {
                id: "backup".to_string(),
                name: None,
                file: None,
                description: None,
            },
        ],
        active_workflow_id: Some("backup".to_string()),
    })
    .expect("raw workflow subset should map into a valid workflow index");

    let active = workflow_index
        .active()
        .expect("active workflow should be set");
    assert_eq!(
        active.id(),
        &WorkflowId::new("backup").expect("non-blank workflow id")
    );
    assert!(active.contains_route(&RouteId::new("backup").expect("non-blank route id")));
}

#[test]
fn rejects_unknown_active_workflow() {
    let error = map_workflow_index(&RawWorkflowIndex {
        workflows: vec![RawWorkflowIndexEntry {
            id: "primary".to_string(),
            name: None,
            file: None,
            description: None,
        }],
        active_workflow_id: Some("missing".to_string()),
    })
    .expect_err("active workflow id must match indexed workflows");

    assert_eq!(
        error,
        InfrastructureAclError::Domain(DomainError::ActiveWorkflowNotFound {
            workflow_id: WorkflowId::new("missing").expect("non-blank workflow id"),
        })
    );
}

#[test]
fn rejects_active_workflow_when_index_is_empty() {
    let error = map_workflow_index(&RawWorkflowIndex {
        workflows: Vec::new(),
        active_workflow_id: Some("primary".to_string()),
    })
    .expect_err("active workflow cannot be set without indexed workflows");

    assert_eq!(
        error,
        InfrastructureAclError::Domain(DomainError::ActiveWorkflowDefinedWithoutWorkflows {
            workflow_id: WorkflowId::new("primary").expect("non-blank workflow id"),
        })
    );
}

#[test]
fn rejects_duplicate_workflow_ids() {
    let error = map_workflow_index(&RawWorkflowIndex {
        workflows: vec![
            RawWorkflowIndexEntry {
                id: "primary".to_string(),
                name: None,
                file: None,
                description: None,
            },
            RawWorkflowIndexEntry {
                id: "primary".to_string(),
                name: None,
                file: None,
                description: None,
            },
        ],
        active_workflow_id: None,
    })
    .expect_err("workflow ids must be unique");

    assert_eq!(
        error,
        InfrastructureAclError::Domain(DomainError::DuplicateWorkflowId {
            workflow_id: WorkflowId::new("primary").expect("non-blank workflow id"),
        })
    );
}

#[test]
fn rejects_duplicate_provider_ids() {
    let error = map_gateway_catalog(
        &[
            RawProvider {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
            },
            RawProvider {
                id: "openai".to_string(),
                name: "Duplicate".to_string(),
            },
        ],
        &[],
    )
    .expect_err("provider ids must be unique");

    assert_eq!(
        error,
        InfrastructureAclError::Domain(DomainError::DuplicateProviderId {
            provider_id: ProviderId::new("openai").expect("non-blank provider id"),
        })
    );
}

#[test]
fn rejects_blank_provider_name_in_gateway_mapping() {
    let error = map_gateway_catalog(
        &[RawProvider {
            id: "openai".to_string(),
            name: "   ".to_string(),
        }],
        &[],
    )
    .expect_err("provider name cannot be blank");

    assert_eq!(
        error,
        InfrastructureAclError::Domain(DomainError::BlankProviderName)
    );
}

#[test]
fn parses_and_maps_minimal_toml_subset() {
    let raw = parse_raw_gateway_config(
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
    .expect("minimal toml subset should parse");

    let (gateway_catalog, workflow_index) =
        map_gateway_config(&raw).expect("parsed raw subset should map");

    assert!(
        gateway_catalog
            .providers()
            .contains(&ProviderId::new("openai").expect("non-blank provider id"))
    );
    assert_eq!(
        workflow_index.active_id(),
        Some(&WorkflowId::new("chat-routing").expect("non-blank workflow id"))
    );
}

#[test]
fn rejects_missing_active_workflow_id_when_workflows_exist() {
    let raw = parse_raw_gateway_config(
        r#"
[[workflows]]
id = "chat-routing"
file = "chat-routing.toml"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw)
        .expect_err("active_workflow_id should be required when workflows exist");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "active_workflow_id must be set when workflows are present".to_string()
        )
    );
}

#[test]
fn rejects_missing_workflow_file_in_gateway_mapping() {
    let raw = parse_raw_gateway_config(
        r#"
active_workflow_id = "chat-routing"

[[workflows]]
id = "chat-routing"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("workflow file is required");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "workflow 'chat-routing' file cannot be empty".to_string()
        )
    );
}

#[test]
fn reports_toml_deserialize_errors() {
    let error =
        parse_raw_gateway_config("providers = [").expect_err("invalid toml should fail parsing");

    match error {
        InfrastructureAclError::TomlDeserialize(message) => {
            assert!(!message.is_empty(), "parse error message should be present");
        }
        other => panic!("expected toml deserialize error, got: {other:?}"),
    }
}

#[test]
fn rejects_route_with_missing_provider_reference() {
    let raw = parse_raw_gateway_config(
        r#"
[[routes]]
id = "chat-route"
match = "/v1/chat/completions"
provider_id = "missing"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("route provider must exist");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "route 'chat-route' references missing provider 'missing'".to_string()
        )
    );
}

#[test]
fn rejects_global_header_rule_with_target_id() {
    let raw = parse_raw_gateway_config(
        r#"
[[header_rules]]
id = "rule-1"
scope = "global"
target_id = "openai"

[[header_rules.actions]]
type = "set"
name = "x-api-key"
value = "demo"
"#,
    )
    .expect("raw config should parse");

    let error =
        map_gateway_config(&raw).expect_err("global header rules must not provide target_id");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "header_rule 'rule-1' must not define target_id for global scope".to_string()
        )
    );
}

#[test]
fn rejects_scoped_header_rule_without_target() {
    let raw = parse_raw_gateway_config(
        r#"
[[providers]]
id = "openai"
name = "OpenAI"

[[header_rules]]
id = "rule-1"
scope = "provider"

[[header_rules.actions]]
type = "remove"
name = "x-remove"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("scoped header rules require target_id");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "header_rule 'rule-1' requires target_id for provider scope".to_string()
        )
    );
}

#[test]
fn rejects_header_rule_without_actions() {
    let raw = parse_raw_gateway_config(
        r#"
[[header_rules]]
id = "rule-1"
scope = "global"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("header rule must include actions");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "header_rule 'rule-1' must contain at least one action".to_string()
        )
    );
}

#[test]
fn validates_basic_rule_graph_subset() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "end"
type = "end"

[[rule_graph.edges]]
id = "edge-1"
source = "start"
target = "end"
"#,
    )
    .expect("raw config should parse");

    let result = map_gateway_config(&raw);
    assert!(result.is_ok(), "basic rule_graph subset should validate");
}

#[test]
fn rejects_rule_graph_with_missing_start_node_reference() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "missing"

[[rule_graph.nodes]]
id = "start"
type = "start"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("start node reference must exist");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph start node 'missing' does not exist".to_string()
        )
    );
}

#[test]
fn rejects_rule_graph_without_single_start_node() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "end"

[[rule_graph.nodes]]
id = "end"
type = "end"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("exactly one start node is required");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph requires exactly one start node, found 0".to_string()
        )
    );
}

#[test]
fn rejects_rule_graph_edge_with_missing_target() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.edges]]
id = "edge-1"
source = "start"
target = "missing"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("edge target must exist");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph edge 'edge-1' missing target 'missing'".to_string()
        )
    );
}

#[test]
fn rejects_select_model_node_with_missing_config() {
    let raw = parse_raw_gateway_config(
        r#"
[[providers]]
id = "openai"
name = "OpenAI"

[[models]]
id = "gpt-4o"
name = "GPT-4o"
provider_id = "openai"

[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "select-model"
type = "select_model"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("select_model config is required");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'select-model' missing select_model config".to_string()
        )
    );
}

#[test]
fn rejects_select_model_node_with_provider_model_mismatch() {
    let raw = parse_raw_gateway_config(
        r#"
[[providers]]
id = "openai"
name = "OpenAI"

[[providers]]
id = "anthropic"
name = "Anthropic"

[[models]]
id = "gpt-4o"
name = "GPT-4o"
provider_id = "openai"

[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "select-model"
type = "select_model"
[rule_graph.nodes.select_model]
provider_id = "anthropic"
model_id = "gpt-4o"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("model-provider ownership must match");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'select-model' model 'gpt-4o' does not belong to provider 'anthropic'"
                .to_string()
        )
    );
}

#[test]
fn rejects_router_node_without_rules() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "router"
type = "router"
[rule_graph.nodes.router]
rules = []
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("router must contain rules");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'router' must define at least one router rule".to_string()
        )
    );
}

#[test]
fn rejects_router_rule_with_missing_target() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "router"
type = "router"
[[rule_graph.nodes.router.rules]]
id = "r1"
target_node_id = "missing"
[[rule_graph.nodes.router.rules.clauses]]
source = "path"
operator = "contains"
value = "/v1"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("router target node must exist");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'router' router rule 'r1' references missing target 'missing'"
                .to_string()
        )
    );
}

#[test]
fn rejects_condition_node_without_config() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "c1"
type = "condition"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("condition config is required");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'c1' missing condition config".to_string()
        )
    );
}

#[test]
fn rejects_condition_expression_mode_without_expression() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "c1"
type = "condition"
[rule_graph.nodes.condition]
mode = "expression"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("expression mode requires expression");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph condition node 'c1' requires expression".to_string()
        )
    );
}

#[test]
fn rejects_condition_builder_mode_with_incomplete_fields() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "c1"
type = "condition"
[rule_graph.nodes.condition]
mode = "builder"
[rule_graph.nodes.condition.builder]
field = "method"
operator = ""
value = "POST"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("builder fields must be complete");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph condition node 'c1' builder fields cannot be empty".to_string()
        )
    );
}

#[test]
fn rejects_condition_node_with_more_than_two_outgoing_edges() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "c1"
type = "condition"
[rule_graph.nodes.condition]
mode = "expression"
expression = "true"

[[rule_graph.nodes]]
id = "n1"
type = "end"

[[rule_graph.nodes]]
id = "n2"
type = "end"

[[rule_graph.nodes]]
id = "n3"
type = "end"

[[rule_graph.edges]]
id = "e1"
source = "c1"
target = "n1"

[[rule_graph.edges]]
id = "e2"
source = "c1"
target = "n2"

[[rule_graph.edges]]
id = "e3"
source = "c1"
target = "n3"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("condition supports at most two edges");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph condition node 'c1' supports at most 2 outgoing edges".to_string()
        )
    );
}

#[test]
fn rejects_log_node_without_config() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "log1"
type = "log"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("log config is required");
    assert_eq!(
        error,
        InfrastructureAclError::Validation("rule_graph node 'log1' missing log config".to_string())
    );
}

#[test]
fn rejects_set_header_node_with_empty_name_or_value() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "set1"
type = "set_header"
[rule_graph.nodes.set_header]
name = "x-key"
value = ""
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("set_header requires non-empty fields");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'set1' header name/value cannot be empty".to_string()
        )
    );
}

#[test]
fn rejects_remove_header_node_without_name() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "rm1"
type = "remove_header"
[rule_graph.nodes.remove_header]
name = " "
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("remove_header requires name");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'rm1' header name cannot be empty".to_string()
        )
    );
}

#[test]
fn rejects_copy_header_node_with_missing_fields() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "copy1"
type = "copy_header"
[rule_graph.nodes.copy_header]
from = "x-src"
to = ""
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("copy_header requires both fields");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'copy1' copy header fields cannot be empty".to_string()
        )
    );
}

#[test]
fn rejects_wasm_plugin_node_without_config() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "wp1"
type = "wasm_plugin"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("wasm_plugin config is required");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'wp1' missing wasm_plugin config".to_string()
        )
    );
}

#[test]
fn rejects_wasm_plugin_node_with_empty_plugin_id() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "wp1"
type = "wasm_plugin"
[rule_graph.nodes.wasm_plugin]
plugin_id = " "
timeout_ms = 20
max_memory_bytes = 1024
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("plugin_id must be non-empty");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'wp1' plugin_id cannot be empty".to_string()
        )
    );
}

#[test]
fn rejects_wasm_plugin_node_with_zero_timeout() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "wp1"
type = "wasm_plugin"
[rule_graph.nodes.wasm_plugin]
plugin_id = "plugin.demo"
timeout_ms = 0
max_memory_bytes = 1024
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("timeout must be positive");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'wp1' timeout_ms must be greater than zero".to_string()
        )
    );
}

#[test]
fn rejects_wasm_plugin_node_with_zero_max_memory() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "wp1"
type = "wasm_plugin"
[rule_graph.nodes.wasm_plugin]
plugin_id = "plugin.demo"
timeout_ms = 20
max_memory_bytes = 0
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("max_memory_bytes must be positive");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'wp1' max_memory_bytes must be greater than zero".to_string()
        )
    );
}

#[test]
fn rejects_wasm_plugin_fs_dirs_without_fs_capability() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "wp1"
type = "wasm_plugin"
[rule_graph.nodes.wasm_plugin]
plugin_id = "plugin.demo"
timeout_ms = 20
max_memory_bytes = 1024
read_dirs = ["sandbox"]
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("fs directories require fs capability grant");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'wp1' fs directories require an fs capability grant".to_string()
        )
    );
}

#[test]
fn rejects_wasm_plugin_fs_capability_without_dirs() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "wp1"
type = "wasm_plugin"
[rule_graph.nodes.wasm_plugin]
plugin_id = "plugin.demo"
timeout_ms = 20
max_memory_bytes = 1024
granted_capabilities = ["fs"]
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("fs capability should require dirs");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'wp1' fs capability requires read_dirs or write_dirs".to_string()
        )
    );
}

#[test]
fn rejects_wasm_plugin_read_dirs_with_absolute_path() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "wp1"
type = "wasm_plugin"
[rule_graph.nodes.wasm_plugin]
plugin_id = "plugin.demo"
timeout_ms = 20
max_memory_bytes = 1024
granted_capabilities = ["fs"]
read_dirs = ["/etc"]
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("read_dirs must be relative");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'wp1' read_dirs must use relative paths".to_string()
        )
    );
}

#[test]
fn rejects_wasm_plugin_read_dirs_with_parent_traversal() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "wp1"
type = "wasm_plugin"
[rule_graph.nodes.wasm_plugin]
plugin_id = "plugin.demo"
timeout_ms = 20
max_memory_bytes = 1024
granted_capabilities = ["fs"]
read_dirs = ["../secret"]
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("read_dirs must not contain parent traversal");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'wp1' read_dirs must not contain parent traversal".to_string()
        )
    );
}

#[test]
fn rejects_wasm_plugin_network_capability_without_allowed_hosts() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "wp1"
type = "wasm_plugin"
[rule_graph.nodes.wasm_plugin]
plugin_id = "plugin.demo"
timeout_ms = 20
max_memory_bytes = 1024
granted_capabilities = ["network"]
"#,
    )
    .expect("raw config should parse");

    let error =
        map_gateway_config(&raw).expect_err("network capability should require allowed_hosts");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'wp1' network capability requires allowed_hosts".to_string()
        )
    );
}

#[test]
fn rejects_wasm_plugin_allowed_hosts_without_network_capability() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "wp1"
type = "wasm_plugin"
[rule_graph.nodes.wasm_plugin]
plugin_id = "plugin.demo"
timeout_ms = 20
max_memory_bytes = 1024
allowed_hosts = ["api.openai.com"]
"#,
    )
    .expect("raw config should parse");

    let error =
        map_gateway_config(&raw).expect_err("allowed_hosts require network capability grant");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'wp1' allowed_hosts require a network capability grant".to_string()
        )
    );
}

#[test]
fn rejects_match_node_without_config() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "m1"
type = "match"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("match config is required");
    assert_eq!(
        error,
        InfrastructureAclError::Validation("rule_graph node 'm1' missing match config".to_string())
    );
}

#[test]
fn rejects_match_node_without_branches() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "end"
type = "end"

[[rule_graph.nodes]]
id = "m1"
type = "match"
[rule_graph.nodes.match]
plugin_id = "plugin.match"
timeout_ms = 20
max_memory_bytes = 1024
branches = []
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("match node must define branches");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'm1' must define at least one match branch".to_string()
        )
    );
}

#[test]
fn rejects_match_branch_with_missing_target() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "m1"
type = "match"
[rule_graph.nodes.match]
plugin_id = "plugin.match"
timeout_ms = 20
max_memory_bytes = 1024
[[rule_graph.nodes.match.branches]]
id = "b1"
expr = "true"
target_node_id = "missing"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("match branch target must exist");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'm1' match branch 'b1' references missing target 'missing'"
                .to_string()
        )
    );
}

#[test]
fn rejects_match_node_with_missing_fallback_target() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "end"
type = "end"

[[rule_graph.nodes]]
id = "m1"
type = "match"
[rule_graph.nodes.match]
plugin_id = "plugin.match"
timeout_ms = 20
max_memory_bytes = 1024
fallback_node_id = "missing"
[[rule_graph.nodes.match.branches]]
id = "b1"
expr = "true"
target_node_id = "end"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("fallback target must exist");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'm1' references missing fallback target 'missing'".to_string()
        )
    );
}

#[test]
fn rejects_code_runner_node_without_config() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "runner1"
type = "code_runner"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("code_runner config is required");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'runner1' missing code_runner config".to_string()
        )
    );
}

#[test]
fn rejects_code_runner_node_with_zero_timeout() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "runner1"
type = "code_runner"
[rule_graph.nodes.code_runner]
timeout_ms = 0
max_memory_bytes = 1024
code = "return {};"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("timeout must be positive");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'runner1' timeout_ms must be greater than zero".to_string()
        )
    );
}

#[test]
fn rejects_code_runner_node_with_zero_max_memory() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "runner1"
type = "code_runner"
[rule_graph.nodes.code_runner]
timeout_ms = 20
max_memory_bytes = 0
code = "return {};"
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("max_memory_bytes must be positive");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'runner1' max_memory_bytes must be greater than zero".to_string()
        )
    );
}

#[test]
fn rejects_code_runner_node_with_empty_code() {
    let raw = parse_raw_gateway_config(
        r#"
[rule_graph]
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"

[[rule_graph.nodes]]
id = "runner1"
type = "code_runner"
[rule_graph.nodes.code_runner]
timeout_ms = 20
max_memory_bytes = 1024
code = " "
"#,
    )
    .expect("raw config should parse");

    let error = map_gateway_config(&raw).expect_err("code cannot be empty");
    assert_eq!(
        error,
        InfrastructureAclError::Validation(
            "rule_graph node 'runner1' code cannot be empty".to_string()
        )
    );
}
