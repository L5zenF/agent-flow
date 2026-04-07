#[cfg(test)]
mod tests {
    use super::super::{
        activate_workflow, create_workflow, get_workflows, put_workflow, validate_config_handler,
        AdminState, CreateWorkflowRequest,
    };
    use crate::config::{parse_config, runtime_state_from_config, GatewayConfig};
    use axum::extract::{Path, State};
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::Json;
    use infrastructure::plugin_registry::load_plugin_registry;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::sync::RwLock;

    fn temp_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be monotonic enough for tests")
            .as_nanos();
        dir.push(format!(
            "proxy-tools-admin-api-{name}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp dir should be creatable");
        dir
    }

    fn build_state(root: &std::path::Path) -> AdminState {
        let config_path = root.join("gateway.toml");
        let config = parse_config(
            r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }
"#,
        )
        .expect("legacy config should parse");
        let runtime_state =
            runtime_state_from_config(&config_path, config).expect("runtime state should build");

        AdminState {
            runtime_state: Arc::new(RwLock::new(runtime_state)),
            config_path,
            plugin_registry: Arc::new(
                load_plugin_registry(&root.join("plugins")).expect("plugin registry should load"),
            ),
        }
    }

    fn write_workflow(root: &std::path::Path, file: &str, node_count: usize, edge_count: usize) {
        let workflows_dir = root.join("workflows");
        fs::create_dir_all(&workflows_dir).expect("workflows dir should be creatable");

        let mut raw = String::from(
            r#"[workflow]
version = 1
start_node_id = "start"

[[workflow.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }
"#,
        );

        for index in 1..node_count {
            raw.push_str(&format!(
                r#"

[[workflow.nodes]]
id = "node-{index}"
type = "end"
position = {{ x = {x}, y = 0.0 }}
"#,
                x = (index as f64) * 120.0
            ));
        }

        for index in 0..edge_count {
            let target = if index + 1 < node_count {
                format!("node-{}", index + 1)
            } else {
                "start".to_string()
            };
            let source = if index == 0 {
                "start".to_string()
            } else {
                format!("node-{index}")
            };
            raw.push_str(&format!(
                r#"

[[workflow.edges]]
id = "edge-{index}"
source = "{source}"
target = "{target}"
"#
            ));
        }

        fs::write(workflows_dir.join(file), raw).expect("workflow file should be writable");
    }

    fn build_indexed_state(root: &std::path::Path) -> AdminState {
        let config_path = root.join("gateway.toml");
        write_workflow(root, "chat-routing.toml", 2, 1);
        write_workflow(root, "fallback.toml", 3, 2);
        let config = parse_config(
            r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
workflows_dir = "workflows"
active_workflow_id = "chat-routing"

[[workflows]]
id = "chat-routing"
name = "Chat Routing"
file = "chat-routing.toml"
description = "Primary chat flow"

[[workflows]]
id = "fallback"
name = "Fallback"
file = "fallback.toml"
description = "Fallback workflow"
"#,
        )
        .expect("indexed config should parse");
        fs::write(
            &config_path,
            toml::to_string_pretty(&config).expect("config should serialize"),
        )
        .expect("config file should be writable");

        let runtime_state =
            runtime_state_from_config(&config_path, config).expect("runtime state should build");

        AdminState {
            runtime_state: Arc::new(RwLock::new(runtime_state)),
            config_path,
            plugin_registry: Arc::new(
                load_plugin_registry(&root.join("plugins")).expect("plugin registry should load"),
            ),
        }
    }

    #[tokio::test]
    async fn validate_rejects_missing_indexed_workflow_file() {
        let root = temp_dir("validate-missing-workflow");
        let state = build_state(&root);
        let candidate = GatewayConfig {
            listen: "127.0.0.1:9001".to_string(),
            admin_listen: "127.0.0.1:9002".to_string(),
            default_secret_env: None,
            providers: Vec::new(),
            models: Vec::new(),
            routes: Vec::new(),
            header_rules: Vec::new(),
            rule_graph: None,
            workflows_dir: Some("workflows".to_string()),
            active_workflow_id: Some("chat-routing".to_string()),
            workflows: vec![crate::config::WorkflowIndexEntry {
                id: "chat-routing".to_string(),
                name: "Chat Routing".to_string(),
                file: "chat-routing.toml".to_string(),
                description: None,
            }],
        };

        let response = validate_config_handler(State(state), Json(candidate))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn lists_workflow_gallery_summaries() {
        let root = temp_dir("workflow-gallery");
        let state = build_indexed_state(&root);

        let Json(summaries) = get_workflows(State(state)).await;

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].id, "chat-routing");
        assert!(summaries[0].is_active);
        assert_eq!(summaries[0].node_count, 2);
        assert_eq!(summaries[0].edge_count, 1);
        assert_eq!(summaries[1].id, "fallback");
        assert!(!summaries[1].is_active);
        assert_eq!(summaries[1].node_count, 3);
        assert_eq!(summaries[1].edge_count, 2);
    }

    #[tokio::test]
    async fn activates_selected_workflow() {
        let root = temp_dir("activate-workflow");
        let state = build_indexed_state(&root);

        let Json(summary) = activate_workflow(Path("fallback".to_string()), State(state.clone()))
            .await
            .expect("activation should succeed");

        assert_eq!(summary.id, "fallback");
        assert!(summary.is_active);
        let runtime_state = state.runtime_state.read().await;
        assert_eq!(runtime_state.config.active_workflow_id.as_deref(), Some("fallback"));
        assert_eq!(
            runtime_state.workflow_set.active_workflow_id.as_deref(),
            Some("fallback")
        );

        let persisted = fs::read_to_string(root.join("gateway.toml"))
            .expect("config file should remain readable");
        assert!(persisted.contains("active_workflow_id = \"fallback\""));
    }

    #[tokio::test]
    async fn create_workflow_persists_file_and_updates_runtime_state() {
        let root = temp_dir("create-workflow");
        let state = build_indexed_state(&root);

        let summary = create_workflow(
            State(state.clone()),
            Json(CreateWorkflowRequest {
                id: "new-flow".to_string(),
                name: "New Flow".to_string(),
                description: Some("Created in admin API".to_string()),
            }),
        )
        .await
        .expect("create should succeed")
        .1
         .0;

        assert_eq!(summary.id, "new-flow");
        assert!(!summary.is_active);
        let persisted_workflow = fs::read_to_string(root.join("workflows/new-flow.toml"))
            .expect("new workflow file should be written");
        assert!(persisted_workflow.contains("[workflow]"));
        assert!(persisted_workflow.contains("start_node_id = \"start\""));

        let runtime_state = state.runtime_state.read().await;
        assert!(runtime_state
            .config
            .workflows
            .iter()
            .any(|workflow| workflow.id == "new-flow"));
        assert!(runtime_state.workflow_set.by_id.contains_key("new-flow"));
    }

    #[tokio::test]
    async fn create_workflow_materializes_legacy_active_graph() {
        let root = temp_dir("create-workflow-legacy");
        let state = build_state(&root);

        let _ = create_workflow(
            State(state.clone()),
            Json(CreateWorkflowRequest {
                id: "new-flow".to_string(),
                name: "New Flow".to_string(),
                description: None,
            }),
        )
        .await
        .expect("create should succeed for legacy configs");

        let default_workflow = fs::read_to_string(root.join("workflows/default.toml"))
            .expect("legacy active workflow should be materialized");
        assert!(default_workflow.contains("start_node_id = \"start\""));

        let runtime_state = state.runtime_state.read().await;
        assert_eq!(runtime_state.config.workflows.len(), 2);
        assert!(runtime_state.workflow_set.by_id.contains_key("default"));
        assert!(runtime_state.workflow_set.by_id.contains_key("new-flow"));
    }

    #[tokio::test]
    async fn put_workflow_persists_file_contents() {
        let root = temp_dir("put-workflow");
        let state = build_indexed_state(&root);

        let saved = put_workflow(
            Path("fallback".to_string()),
            State(state.clone()),
            Json(crate::config::WorkflowFileConfig {
                workflow: crate::config::RuleGraphConfig {
                    version: 1,
                    start_node_id: "start".to_string(),
                    nodes: vec![
                        crate::config::RuleGraphNode {
                            id: "start".to_string(),
                            node_type: crate::config::RuleGraphNodeType::Start,
                            position: crate::config::GraphPosition { x: 0.0, y: 0.0 },
                            note: None,
                            condition: None,
                            route_provider: None,
                            select_model: None,
                            rewrite_path: None,
                            set_context: None,
                            router: None,
                            log: None,
                            set_header: None,
                            remove_header: None,
                            copy_header: None,
                            set_header_if_absent: None,
                            note_node: None,
                            wasm_plugin: None,
                            match_node: None,
                            code_runner: None,
                        },
                        crate::config::RuleGraphNode {
                            id: "end".to_string(),
                            node_type: crate::config::RuleGraphNodeType::End,
                            position: crate::config::GraphPosition { x: 120.0, y: 0.0 },
                            note: None,
                            condition: None,
                            route_provider: None,
                            select_model: None,
                            rewrite_path: None,
                            set_context: None,
                            router: None,
                            log: None,
                            set_header: None,
                            remove_header: None,
                            copy_header: None,
                            set_header_if_absent: None,
                            note_node: None,
                            wasm_plugin: None,
                            match_node: None,
                            code_runner: None,
                        },
                    ],
                    edges: vec![crate::config::RuleGraphEdge {
                        id: "edge-1".to_string(),
                        source: "start".to_string(),
                        target: "end".to_string(),
                        source_handle: None,
                    }],
                },
            }),
        )
        .await
        .expect("save should succeed")
        .0;

        assert_eq!(saved.workflow.nodes.len(), 2);
        let persisted = fs::read_to_string(root.join("workflows/fallback.toml"))
            .expect("workflow file should remain readable");
        assert!(persisted.contains("id = \"end\""));

        let runtime_state = state.runtime_state.read().await;
        let workflow = runtime_state
            .workflow_set
            .by_id
            .get("fallback")
            .expect("updated workflow should be loaded");
        assert_eq!(workflow.workflow.nodes.len(), 2);
        assert_eq!(workflow.workflow.edges.len(), 1);
    }
}
