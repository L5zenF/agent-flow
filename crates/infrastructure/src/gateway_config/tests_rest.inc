const VALID_CONFIG: &str = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"

[[providers]]
id = "kimi"
name = "Kimi"
base_url = "https://api.kimi.com"

[[providers.default_headers]]
name = "Authorization"
value = "enc:v1:test"
encrypted = true

[[models]]
id = "kimi-k2"
name = "Kimi K2"
provider_id = "kimi"

[[routes]]
id = "chat-default"
priority = 100
enabled = true
matcher = 'path.startsWith("/v1/chat/completions") && method == "POST"'
provider_id = "kimi"
model_id = "kimi-k2"
path_rewrite = "/coding/v1/chat/completions"

[[header_rules]]
id = "inject-model-header"
enabled = true
scope = "model"
target_id = "kimi-k2"
when = 'path.startsWith("/v1/")'

[[header_rules.actions]]
type = "set"
name = "X-Model"
value = "${model.id}"

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }

[[rule_graph.nodes]]
id = "cond-1"
type = "condition"
position = { x = 120.0, y = 0.0 }

[rule_graph.nodes.condition]
mode = "expression"
expression = 'path.startsWith("/v1/")'

[[rule_graph.nodes]]
id = "provider-kimi"
type = "route_provider"
position = { x = 240.0, y = 0.0 }

[rule_graph.nodes.route_provider]
provider_id = "kimi"

[[rule_graph.nodes]]
id = "end"
type = "end"
position = { x = 360.0, y = 0.0 }

[[rule_graph.edges]]
id = "edge-1"
source = "start"
target = "cond-1"

[[rule_graph.edges]]
id = "edge-2"
source = "cond-1"
source_handle = "true"
target = "provider-kimi"

[[rule_graph.edges]]
id = "edge-3"
source = "provider-kimi"
target = "end"
"#;

const VALID_WASM_PLUGIN_CONFIG: &str = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"

[[providers]]
id = "kimi"
name = "Kimi"
base_url = "https://api.kimi.com"

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }

[[rule_graph.nodes]]
id = "plugin"
type = "wasm_plugin"
position = { x = 120.0, y = 0.0 }

[rule_graph.nodes.wasm_plugin]
plugin_id = "intent-classifier"
fuel = 500000
max_memory_bytes = 16777216
granted_capabilities = ["fs", "network"]
read_dirs = ["plugins-data/common"]
write_dirs = ["plugins-data/runtime"]
allowed_hosts = ["api.example.com:443"]

[rule_graph.nodes.wasm_plugin.config]
prompt = "classify request intent"
default_intent = "chat"

[[rule_graph.nodes]]
id = "end"
type = "end"
position = { x = 240.0, y = 0.0 }

[[rule_graph.edges]]
id = "edge-1"
source = "start"
target = "plugin"

[[rule_graph.edges]]
id = "edge-2"
source = "plugin"
target = "end"
"#;

const VALID_WASM_MATCH_CONFIG: &str = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"

[[providers]]
id = "kimi"
name = "Kimi"
base_url = "https://api.kimi.com"

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }

[[rule_graph.nodes]]
id = "matcher"
type = "match"
position = { x = 120.0, y = 0.0 }

[rule_graph.nodes.match]
plugin_id = "remote-policy-router"
fuel = 500000
max_memory_bytes = 16777216
granted_capabilities = ["log"]
fallback_node_id = "end"

[[rule_graph.nodes.match.branches]]
id = "chat"
expr = "ctx.header.x-tenant == enterprise"
target_node_id = "end"

[rule_graph.nodes.match.config]
match_header = "x-tenant"
fallback_port = "default"

[[rule_graph.nodes]]
id = "end"
type = "end"
position = { x = 240.0, y = 0.0 }

[[rule_graph.edges]]
id = "edge-1"
source = "start"
target = "matcher"

[[rule_graph.edges]]
id = "edge-2"
source = "matcher"
source_handle = "default"
target = "end"
"#;

const VALID_CODE_RUNNER_CONFIG: &str = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"

[[providers]]
id = "kimi"
name = "Kimi"
base_url = "https://api.kimi.com"

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }

[[rule_graph.nodes]]
id = "runner"
type = "code_runner"
position = { x = 120.0, y = 0.0 }

[rule_graph.nodes.code_runner]
language = "javascript"
timeout_ms = 25
max_memory_bytes = 16777216
code = "export function run(input) { return {}; }"

[[rule_graph.nodes]]
id = "end"
type = "end"
position = { x = 240.0, y = 0.0 }

[[rule_graph.edges]]
id = "edge-1"
source = "start"
target = "runner"

[[rule_graph.edges]]
id = "edge-2"
source = "runner"
target = "end"
"#;

fn temp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be monotonic enough for tests")
        .as_nanos();
    dir.push(format!(
        "proxy-tools-config-{name}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("temp dir should be creatable");
    dir
}

// Existing gateway_config tests retained during module split.
