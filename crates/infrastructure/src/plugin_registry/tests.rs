use super::manifest::{
    ManifestCapability, ManifestCategory, ManifestIcon, ManifestTone, PluginRuntimeKind,
    parse_plugin_manifest,
};
use super::{load_plugin_registry, resolve_plugins_root};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const TEST_COMPONENT_BYTES: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00, 0x07, 0x10, 0x01, 0x41, 0x02, 0x01, 0x42, 0x00,
    0x04, 0x01, 0x05, 0x65, 0x3a, 0x65, 0x2f, 0x65, 0x05, 0x00, 0x0b, 0x07, 0x01, 0x00, 0x01, 0x65,
    0x03, 0x00, 0x00,
];
const TEST_CORE_MODULE_BYTES: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

fn temp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be monotonic enough for tests")
        .as_nanos();
    dir.push(format!(
        "proxy-tools-wasm-{name}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("temp dir should be creatable");
    dir
}

fn write_plugin(root: &Path, id: &str, manifest: &str) {
    let plugin_dir = root.join(id);
    fs::create_dir_all(&plugin_dir).expect("plugin dir should be creatable");
    fs::write(plugin_dir.join("plugin.toml"), manifest).expect("manifest should write");
    let wasm_dir = plugin_dir.join("wasm");
    fs::create_dir_all(&wasm_dir).expect("wasm dir should be creatable");
    fs::write(wasm_dir.join("plugin.wasm"), TEST_COMPONENT_BYTES).expect("wasm should write");
}

#[test]
fn scans_plugins_directory() {
    let root = temp_dir("scan");
    write_plugin(
        &root,
        "intent-classifier",
        r#"
id = "intent-classifier"
name = "Intent Classifier"
version = "1.0.0"
description = "Classifies request intent"
supported_output_ports = ["chat", "default"]
capabilities = ["log", "network"]
default_config_schema_hints = { prompt = "string", default_intent = "string" }
[ui]
icon = "split"
category = "control"
tone = "violet"
order = 10
tags = ["intent", "branch"]
"#,
    );

    let registry = load_plugin_registry(&root).expect("registry should load");
    assert_eq!(registry.root(), root.as_path());
    assert_eq!(registry.len(), 1);
    assert_eq!(registry.component_cache().len(), 1);

    let plugin = registry
        .get("intent-classifier")
        .expect("plugin should be registered");
    assert_eq!(plugin.plugin_id(), "intent-classifier");
    assert_eq!(plugin.manifest().name, "Intent Classifier");
    assert_eq!(
        plugin.manifest().supported_output_ports,
        vec!["chat", "default"]
    );
    assert_eq!(
        plugin.manifest().capabilities,
        vec![ManifestCapability::Log, ManifestCapability::Network]
    );
    assert_eq!(plugin.manifest().ui.icon, Some(ManifestIcon::Split));
    assert_eq!(
        plugin.manifest().ui.category,
        Some(ManifestCategory::Control)
    );
    assert_eq!(plugin.manifest().ui.tone, Some(ManifestTone::Violet));
    assert_eq!(plugin.manifest().ui.order, Some(10));
    assert_eq!(plugin.manifest().ui.tags, vec!["intent", "branch"]);
    assert_eq!(plugin.directory(), root.join("intent-classifier").as_path());
    assert_eq!(
        plugin.wasm_path(),
        root.join("intent-classifier")
            .join("wasm")
            .join("plugin.wasm")
            .as_path()
    );
    assert_eq!(plugin.runtime_kind(), PluginRuntimeKind::Component);
}

#[test]
fn resolves_plugins_directory_from_project_root() {
    let root = temp_dir("resolve-plugins");
    let config_dir = root.join("config");
    let plugins_dir = root.join("plugins");
    let config_path = config_dir.join("gateway.toml");

    fs::create_dir_all(&config_dir).expect("config dir should be creatable");
    fs::create_dir_all(&plugins_dir).expect("plugins dir should be creatable");
    fs::write(&config_path, "listen = \"127.0.0.1:3000\"\n")
        .expect("config file should be writable");

    let resolved = resolve_plugins_root(&config_path).expect("plugins root should resolve");
    assert_eq!(
        fs::canonicalize(&resolved).expect("resolved path should canonicalize"),
        fs::canonicalize(&plugins_dir).expect("plugins dir should canonicalize")
    );
}

#[test]
fn parses_plugin_toml() {
    let manifest = parse_plugin_manifest(
        Path::new("plugins/intent-classifier/plugin.toml"),
        r#"
id = "intent-classifier"
name = "Intent Classifier"
version = "1.0.0"
description = "Classifies request intent"
supported_output_ports = ["chat", "default"]
capabilities = ["fs", "network"]
default_config_schema_hints = { prompt = "string", default_intent = "string" }
[ui]
icon = "route"
category = "routing"
tone = "sky"
order = 20
tags = ["tenant", "policy"]
"#,
    )
    .expect("manifest should parse");

    assert_eq!(manifest.id, "intent-classifier");
    assert_eq!(manifest.name, "Intent Classifier");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.description, "Classifies request intent");
    assert_eq!(manifest.supported_output_ports, vec!["chat", "default"]);
    assert_eq!(
        manifest.capabilities,
        vec![ManifestCapability::Fs, ManifestCapability::Network]
    );
    assert_eq!(manifest.ui.icon, Some(ManifestIcon::Route));
    assert_eq!(manifest.ui.category, Some(ManifestCategory::Routing));
    assert_eq!(manifest.ui.tone, Some(ManifestTone::Sky));
    assert_eq!(manifest.ui.order, Some(20));
    assert_eq!(manifest.ui.tags, vec!["tenant", "policy"]);
    assert_eq!(manifest.runtime, PluginRuntimeKind::Component);
    let hints = manifest
        .default_config_schema_hints
        .as_ref()
        .and_then(|value| value.as_table())
        .expect("schema hints should be a table");
    assert_eq!(
        hints.get("prompt").and_then(|value| value.as_str()),
        Some("string")
    );
    assert_eq!(
        hints.get("default_intent").and_then(|value| value.as_str()),
        Some("string")
    );
}

#[test]
fn parses_core_runtime_manifest() {
    let manifest = parse_plugin_manifest(
        Path::new("plugins/js-code-runner/plugin.toml"),
        r#"
id = "js-code-runner"
name = "Code Runner"
version = "0.1.0"
description = "Runs javascript inside a core wasm guest"
runtime = "core"
supported_output_ports = ["default"]
capabilities = ["log"]
"#,
    )
    .expect("manifest should parse");

    assert_eq!(manifest.runtime, PluginRuntimeKind::Core);
}

#[test]
fn loads_core_runtime_plugin_registry_entry() {
    let root = temp_dir("core-runtime");
    let plugin_dir = root.join("js-code-runner");
    fs::create_dir_all(&plugin_dir).expect("plugin dir should be creatable");
    fs::write(
        plugin_dir.join("plugin.toml"),
        r#"
id = "js-code-runner"
name = "Code Runner"
version = "0.1.0"
description = "Runs javascript inside a core wasm guest"
runtime = "core"
supported_output_ports = ["default"]
capabilities = ["log"]
"#,
    )
    .expect("manifest should write");
    let wasm_dir = plugin_dir.join("wasm");
    fs::create_dir_all(&wasm_dir).expect("wasm dir should be creatable");
    fs::write(wasm_dir.join("plugin.wasm"), TEST_CORE_MODULE_BYTES).expect("wasm should write");

    let registry = load_plugin_registry(&root).expect("registry should load");
    let plugin = registry
        .get("js-code-runner")
        .expect("core plugin should be registered");
    assert_eq!(plugin.runtime_kind(), PluginRuntimeKind::Core);
}

#[test]
fn rejects_empty_supported_output_port_names() {
    let error = match parse_plugin_manifest(
        Path::new("plugins/intent-classifier/plugin.toml"),
        r#"
id = "intent-classifier"
name = "Intent Classifier"
version = "1.0.0"
description = "Classifies request intent"
supported_output_ports = ["chat", "   "]
capabilities = ["fs"]
"#,
    ) {
        Ok(_) => panic!("empty supported_output_ports should fail"),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("contains an empty supported_output_ports entry"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_duplicate_supported_output_port_names() {
    let error = match parse_plugin_manifest(
        Path::new("plugins/intent-classifier/plugin.toml"),
        r#"
id = "intent-classifier"
name = "Intent Classifier"
version = "1.0.0"
description = "Classifies request intent"
supported_output_ports = ["chat", " chat "]
capabilities = ["network"]
"#,
    ) {
        Ok(_) => panic!("duplicate supported_output_ports should fail"),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("declares duplicate supported_output_port 'chat'"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_duplicate_plugin_ids() {
    let root = temp_dir("duplicate");
    let manifest = r#"
id = "intent-classifier"
name = "Intent Classifier"
version = "1.0.0"
description = "Classifies request intent"
supported_output_ports = ["chat", "default"]
capabilities = ["log"]
"#;
    write_plugin(&root, "intent-classifier", manifest);
    write_plugin(&root, "intent-classifier-copy", manifest);

    let error = match load_plugin_registry(&root) {
        Ok(_) => panic!("duplicate ids should fail"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("duplicate plugin id 'intent-classifier'"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_unsupported_capability_declarations() {
    let root = temp_dir("unsupported-capability");
    write_plugin(
        &root,
        "intent-classifier",
        r#"
id = "intent-classifier"
name = "Intent Classifier"
version = "1.0.0"
description = "Classifies request intent"
supported_output_ports = ["chat", "default"]
capabilities = ["log", "shell"]
"#,
    );

    let error = match load_plugin_registry(&root) {
        Ok(_) => panic!("unsupported capability should fail"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("unsupported capability 'shell' (supported: log, fs, network)"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_unsupported_ui_icon_declarations() {
    let error = match parse_plugin_manifest(
        Path::new("plugins/intent-classifier/plugin.toml"),
        r#"
id = "intent-classifier"
name = "Intent Classifier"
version = "1.0.0"
description = "Classifies request intent"
supported_output_ports = ["chat", "default"]
capabilities = ["log"]
[ui]
icon = "rocket"
"#,
    ) {
        Ok(_) => panic!("unsupported ui.icon should fail"),
        Err(error) => error,
    };

    assert!(
        error.to_string().contains("unsupported ui.icon 'rocket'"),
        "unexpected error: {error}"
    );
}
