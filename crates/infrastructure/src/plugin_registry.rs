use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use wasmtime::{Config as WasmtimeConfig, Engine, Module, component::Component};

type PluginResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestCapability {
    Log,
    Fs,
    Network,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestIcon {
    Puzzle,
    Split,
    Route,
    Wand,
    Shield,
    Code,
    Filter,
    Database,
    FileText,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestCategory {
    Control,
    Transform,
    Routing,
    Policy,
    Utility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestTone {
    Slate,
    Blue,
    Sky,
    Teal,
    Emerald,
    Amber,
    Rose,
    Violet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PluginManifestUi {
    pub icon: Option<ManifestIcon>,
    pub category: Option<ManifestCategory>,
    pub tone: Option<ManifestTone>,
    pub order: Option<i32>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginConfigFieldType {
    Text,
    Textarea,
    Select,
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginConfigFieldDataSource {
    Providers,
    Models,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginConfigField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: PluginConfigFieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub help_text: Option<String>,
    #[serde(default)]
    pub data_source: Option<PluginConfigFieldDataSource>,
    #[serde(default)]
    pub depends_on: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginConfigSchema {
    #[serde(default = "default_config_schema_version")]
    pub version: u32,
    #[serde(default)]
    pub fields: Vec<PluginConfigField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntimeKind {
    #[default]
    Component,
    Core,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub runtime: PluginRuntimeKind,
    pub supported_output_ports: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<ManifestCapability>,
    #[serde(default)]
    pub default_config_schema_hints: Option<toml::Value>,
    #[serde(default)]
    pub config_schema: Option<PluginConfigSchema>,
    #[serde(default)]
    pub ui: PluginManifestUi,
}

#[derive(Debug, Deserialize)]
struct RawPluginManifest {
    id: String,
    name: String,
    version: String,
    description: String,
    #[serde(default)]
    runtime: Option<String>,
    supported_output_ports: Vec<String>,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    default_config_schema_hints: Option<toml::Value>,
    #[serde(default)]
    config_schema: Option<PluginConfigSchema>,
    #[serde(default)]
    ui: RawPluginManifestUi,
}

#[derive(Debug, Deserialize, Default)]
struct RawPluginManifestUi {
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    tone: Option<String>,
    #[serde(default)]
    order: Option<i32>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Clone)]
pub struct ComponentCache {
    components: BTreeMap<PathBuf, Arc<Component>>,
    modules: BTreeMap<PathBuf, Arc<Module>>,
}

impl ComponentCache {
    pub fn new() -> Self {
        Self {
            components: BTreeMap::new(),
            modules: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.components.len() + self.modules.len()
    }

    pub fn get_or_load(&mut self, engine: &Engine, path: &Path) -> PluginResult<Arc<Component>> {
        let cache_key = path.to_path_buf();
        if let Some(component) = self.components.get(&cache_key) {
            return Ok(component.clone());
        }

        let component = Arc::new(Component::from_file(engine, path)?);
        self.components.insert(cache_key, component.clone());
        Ok(component)
    }

    pub fn get_or_load_module(
        &mut self,
        engine: &Engine,
        path: &Path,
    ) -> PluginResult<Arc<Module>> {
        let cache_key = path.to_path_buf();
        if let Some(module) = self.modules.get(&cache_key) {
            return Ok(module.clone());
        }

        let module = Arc::new(Module::from_file(engine, path)?);
        self.modules.insert(cache_key, module.clone());
        Ok(module)
    }
}

#[derive(Clone)]
enum LoadedPluginArtifact {
    Component(Arc<Component>),
    Module(Arc<Module>),
}

#[derive(Clone)]
pub struct LoadedPlugin {
    manifest: PluginManifest,
    manifest_path: PathBuf,
    directory: PathBuf,
    wasm_path: PathBuf,
    artifact: LoadedPluginArtifact,
}

impl LoadedPlugin {
    pub fn plugin_id(&self) -> &str {
        &self.manifest.id
    }

    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn wasm_path(&self) -> &Path {
        &self.wasm_path
    }

    pub fn component(&self) -> &Component {
        match &self.artifact {
            LoadedPluginArtifact::Component(component) => component.as_ref(),
            LoadedPluginArtifact::Module(_) => panic!(
                "plugin '{}' is registered as a core wasm module",
                self.manifest.id
            ),
        }
    }

    pub fn module(&self) -> &Module {
        match &self.artifact {
            LoadedPluginArtifact::Component(_) => {
                panic!("plugin '{}' is registered as a component", self.manifest.id)
            }
            LoadedPluginArtifact::Module(module) => module.as_ref(),
        }
    }

    pub fn runtime_kind(&self) -> PluginRuntimeKind {
        self.manifest.runtime
    }
}

#[derive(Clone)]
pub struct PluginRegistry {
    plugins_root: PathBuf,
    engine: Arc<Engine>,
    component_cache: ComponentCache,
    plugins: BTreeMap<String, LoadedPlugin>,
}

impl PluginRegistry {
    pub fn root(&self) -> &Path {
        &self.plugins_root
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn engine(&self) -> &Engine {
        self.engine.as_ref()
    }

    pub fn component_cache(&self) -> &ComponentCache {
        &self.component_cache
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    pub fn get(&self, plugin_id: &str) -> Option<&LoadedPlugin> {
        self.plugins.get(plugin_id)
    }

    pub fn plugins(&self) -> impl Iterator<Item = &LoadedPlugin> {
        self.plugins.values()
    }
}

pub fn load_plugin_registry(path: &Path) -> PluginResult<PluginRegistry> {
    let engine = Arc::new(build_component_engine()?);

    if !path.exists() {
        return Ok(PluginRegistry {
            plugins_root: path.to_path_buf(),
            engine,
            component_cache: ComponentCache::new(),
            plugins: BTreeMap::new(),
        });
    }

    if !path.is_dir() {
        return Err(invalid_data(format!(
            "plugin registry root '{}' is not a directory",
            path.display()
        )));
    }

    let mut component_cache = ComponentCache::new();
    let mut plugins = BTreeMap::new();
    let mut entries = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let directory = entry.path();
        let manifest_path = directory.join("plugin.toml");
        let wasm_path = resolve_plugin_wasm_path(&directory)?;

        if !manifest_path.is_file() {
            return Err(invalid_data(format!(
                "plugin directory '{}' is missing plugin.toml",
                directory.display()
            )));
        }

        let manifest_raw = fs::read_to_string(&manifest_path)?;
        let manifest = parse_plugin_manifest(&manifest_path, &manifest_raw)?;

        let artifact = match manifest.runtime {
            PluginRuntimeKind::Component => LoadedPluginArtifact::Component(
                component_cache.get_or_load(engine.as_ref(), &wasm_path)?,
            ),
            PluginRuntimeKind::Core => LoadedPluginArtifact::Module(
                component_cache.get_or_load_module(engine.as_ref(), &wasm_path)?,
            ),
        };
        let plugin = LoadedPlugin {
            manifest: manifest.clone(),
            manifest_path,
            directory,
            wasm_path,
            artifact,
        };

        if plugins.insert(manifest.id.clone(), plugin).is_some() {
            return Err(invalid_data(format!(
                "duplicate plugin id '{}'",
                manifest.id
            )));
        }
    }

    Ok(PluginRegistry {
        plugins_root: path.to_path_buf(),
        engine,
        component_cache,
        plugins,
    })
}

pub fn resolve_plugins_root(config_path: &Path) -> PluginResult<PathBuf> {
    let canonical_config_path = fs::canonicalize(config_path)?;
    let config_dir = canonical_config_path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "config path '{}' does not have a parent directory",
                canonical_config_path.display()
            ),
        )
    })?;

    let mut candidates = vec![config_dir.join("plugins")];
    if let Some(project_root) = config_dir.parent() {
        let project_plugins = project_root.join("plugins");
        if project_plugins != candidates[0] {
            candidates.push(project_plugins);
        }
    }

    let checked_locations = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    for candidate in candidates {
        if candidate.is_dir() {
            return Ok(candidate);
        }
        if candidate.exists() {
            return Err(invalid_data(format!(
                "resolved plugins path '{}' exists but is not a directory",
                candidate.display()
            )));
        }
    }

    Err(invalid_data(format!(
        "could not resolve plugins directory for config '{}'; checked: {}",
        canonical_config_path.display(),
        checked_locations.join(", ")
    )))
}

fn resolve_plugin_wasm_path(directory: &Path) -> PluginResult<PathBuf> {
    let nested_path = directory.join("wasm").join("plugin.wasm");
    if nested_path.is_file() {
        return Ok(nested_path);
    }

    let legacy_path = directory.join("plugin.wasm");
    if legacy_path.is_file() {
        return Ok(legacy_path);
    }

    Err(invalid_data(format!(
        "plugin directory '{}' is missing wasm/plugin.wasm (and legacy plugin.wasm)",
        directory.display()
    )))
}

fn parse_plugin_manifest(path: &Path, raw: &str) -> PluginResult<PluginManifest> {
    let raw_manifest: RawPluginManifest = toml::from_str(raw).map_err(|error| {
        invalid_data(format!(
            "failed to parse plugin manifest '{}': {error}",
            path.display()
        ))
    })?;

    let capabilities = raw_manifest
        .capabilities
        .into_iter()
        .map(|capability| parse_manifest_capability(path, &capability))
        .collect::<PluginResult<Vec<_>>>()?;
    let supported_output_ports =
        validate_supported_output_ports(path, raw_manifest.supported_output_ports)?;
    let ui = parse_manifest_ui(path, raw_manifest.ui)?;

    Ok(PluginManifest {
        id: raw_manifest.id,
        name: raw_manifest.name,
        version: raw_manifest.version,
        description: raw_manifest.description,
        runtime: parse_plugin_runtime(path, raw_manifest.runtime.as_deref())?,
        supported_output_ports,
        capabilities,
        default_config_schema_hints: raw_manifest.default_config_schema_hints,
        config_schema: raw_manifest.config_schema,
        ui,
    })
}

fn build_component_engine() -> PluginResult<Engine> {
    let mut config = WasmtimeConfig::new();
    config.wasm_component_model(true);
    config.consume_fuel(true);
    config.epoch_interruption(true);
    Engine::new(&config).map_err(Into::into)
}

fn parse_manifest_capability(path: &Path, capability: &str) -> PluginResult<ManifestCapability> {
    match capability {
        "log" => Ok(ManifestCapability::Log),
        "fs" => Ok(ManifestCapability::Fs),
        "network" => Ok(ManifestCapability::Network),
        other => Err(invalid_data(format!(
            "plugin manifest '{}' declares unsupported capability '{}' (supported: log, fs, network)",
            path.display(),
            other
        ))),
    }
}

fn parse_plugin_runtime(path: &Path, runtime: Option<&str>) -> PluginResult<PluginRuntimeKind> {
    match runtime.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(PluginRuntimeKind::Component),
        Some("component") => Ok(PluginRuntimeKind::Component),
        Some("core") => Ok(PluginRuntimeKind::Core),
        Some(other) => Err(invalid_data(format!(
            "plugin manifest '{}' declares unsupported runtime '{}' (supported: component, core)",
            path.display(),
            other
        ))),
    }
}

fn parse_manifest_ui(path: &Path, raw_ui: RawPluginManifestUi) -> PluginResult<PluginManifestUi> {
    let icon = raw_ui
        .icon
        .as_deref()
        .map(|value| parse_manifest_icon(path, value))
        .transpose()?;
    let category = raw_ui
        .category
        .as_deref()
        .map(|value| parse_manifest_category(path, value))
        .transpose()?;
    let tone = raw_ui
        .tone
        .as_deref()
        .map(|value| parse_manifest_tone(path, value))
        .transpose()?;
    let tags = raw_ui
        .tags
        .into_iter()
        .map(|tag| normalize_manifest_tag(path, tag))
        .collect::<PluginResult<Vec<_>>>()?;

    Ok(PluginManifestUi {
        icon,
        category,
        tone,
        order: raw_ui.order,
        tags,
    })
}

fn parse_manifest_icon(path: &Path, value: &str) -> PluginResult<ManifestIcon> {
    match value {
        "puzzle" => Ok(ManifestIcon::Puzzle),
        "split" => Ok(ManifestIcon::Split),
        "route" => Ok(ManifestIcon::Route),
        "wand" => Ok(ManifestIcon::Wand),
        "shield" => Ok(ManifestIcon::Shield),
        "code" => Ok(ManifestIcon::Code),
        "filter" => Ok(ManifestIcon::Filter),
        "database" => Ok(ManifestIcon::Database),
        "file_text" => Ok(ManifestIcon::FileText),
        other => Err(invalid_data(format!(
            "plugin manifest '{}' declares unsupported ui.icon '{}' (supported: puzzle, split, route, wand, shield, code, filter, database, file_text)",
            path.display(),
            other
        ))),
    }
}

fn parse_manifest_category(path: &Path, value: &str) -> PluginResult<ManifestCategory> {
    match value {
        "control" => Ok(ManifestCategory::Control),
        "transform" => Ok(ManifestCategory::Transform),
        "routing" => Ok(ManifestCategory::Routing),
        "policy" => Ok(ManifestCategory::Policy),
        "utility" => Ok(ManifestCategory::Utility),
        other => Err(invalid_data(format!(
            "plugin manifest '{}' declares unsupported ui.category '{}' (supported: control, transform, routing, policy, utility)",
            path.display(),
            other
        ))),
    }
}

fn parse_manifest_tone(path: &Path, value: &str) -> PluginResult<ManifestTone> {
    match value {
        "slate" => Ok(ManifestTone::Slate),
        "blue" => Ok(ManifestTone::Blue),
        "sky" => Ok(ManifestTone::Sky),
        "teal" => Ok(ManifestTone::Teal),
        "emerald" => Ok(ManifestTone::Emerald),
        "amber" => Ok(ManifestTone::Amber),
        "rose" => Ok(ManifestTone::Rose),
        "violet" => Ok(ManifestTone::Violet),
        other => Err(invalid_data(format!(
            "plugin manifest '{}' declares unsupported ui.tone '{}' (supported: slate, blue, sky, teal, emerald, amber, rose, violet)",
            path.display(),
            other
        ))),
    }
}

fn normalize_manifest_tag(path: &Path, tag: String) -> PluginResult<String> {
    let normalized = tag.trim();
    if normalized.is_empty() {
        return Err(invalid_data(format!(
            "plugin manifest '{}' contains an empty ui.tags entry",
            path.display()
        )));
    }
    Ok(normalized.to_owned())
}

fn validate_supported_output_ports(path: &Path, ports: Vec<String>) -> PluginResult<Vec<String>> {
    let mut seen = BTreeSet::new();
    let mut validated = Vec::with_capacity(ports.len());

    for port in ports {
        let normalized = port.trim();
        if normalized.is_empty() {
            return Err(invalid_data(format!(
                "plugin manifest '{}' contains an empty supported_output_ports entry",
                path.display()
            )));
        }
        if !seen.insert(normalized.to_owned()) {
            return Err(invalid_data(format!(
                "plugin manifest '{}' declares duplicate supported_output_port '{}'",
                path.display(),
                normalized
            )));
        }
        validated.push(normalized.to_owned());
    }

    Ok(validated)
}

fn invalid_data(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message.into(),
    ))
}

fn default_config_schema_version() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::{
        ManifestCapability, ManifestCategory, ManifestIcon, ManifestTone, PluginRuntimeKind,
        load_plugin_registry, parse_plugin_manifest, resolve_plugins_root,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    const TEST_COMPONENT_BYTES: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00, 0x07, 0x10, 0x01, 0x41, 0x02, 0x01, 0x42,
        0x00, 0x04, 0x01, 0x05, 0x65, 0x3a, 0x65, 0x2f, 0x65, 0x05, 0x00, 0x0b, 0x07, 0x01, 0x00,
        0x01, 0x65, 0x03, 0x00, 0x00,
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
}
