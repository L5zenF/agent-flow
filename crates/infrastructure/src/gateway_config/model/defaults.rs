pub(crate) fn default_listen() -> String {
    "127.0.0.1:9001".to_string()
}

pub(crate) fn default_admin_listen() -> String {
    "127.0.0.1:9002".to_string()
}

pub(crate) fn default_rule_graph_version() -> u32 {
    1
}

pub(crate) fn default_wasm_plugin_timeout_ms() -> u64 {
    20
}

pub(crate) fn default_enabled() -> bool {
    true
}
