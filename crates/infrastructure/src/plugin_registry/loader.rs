use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use wasmtime::{Config as WasmtimeConfig, Engine};

use super::cache::ComponentCache;
use super::manifest::{PluginManifest, PluginRuntimeKind, parse_plugin_manifest};
use super::paths::{invalid_data, resolve_plugin_wasm_path};
use super::registry::{LoadedPlugin, PluginRegistry};

type PluginResult<T> = Result<T, Box<dyn Error>>;

pub fn load_plugin_registry(path: &Path) -> PluginResult<PluginRegistry> {
    PluginRegistryLoader::new(path)?.load()
}

struct PluginRegistryLoader<'a> {
    root: &'a Path,
    engine: Arc<Engine>,
    cache: ComponentCache,
    plugins: BTreeMap<String, LoadedPlugin>,
}

impl<'a> PluginRegistryLoader<'a> {
    fn new(root: &'a Path) -> PluginResult<Self> {
        Ok(Self {
            root,
            engine: Arc::new(build_component_engine()?),
            cache: ComponentCache::new(),
            plugins: BTreeMap::new(),
        })
    }

    fn load(mut self) -> PluginResult<PluginRegistry> {
        if !self.root.exists() {
            return Ok(PluginRegistry::empty(
                self.root.to_path_buf(),
                self.engine.clone(),
            ));
        }

        if !self.root.is_dir() {
            return Err(invalid_data(format!(
                "plugin registry root '{}' is not a directory",
                self.root.display()
            )));
        }

        let mut entries = fs::read_dir(self.root)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            if !entry.file_type()?.is_dir() {
                continue;
            }
            self.load_plugin_dir(entry.path())?;
        }

        Ok(PluginRegistry::loaded(
            self.root.to_path_buf(),
            self.engine,
            self.cache,
            self.plugins,
        ))
    }

    fn load_plugin_dir(&mut self, directory: std::path::PathBuf) -> PluginResult<()> {
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
        let plugin = self.build_plugin(manifest, manifest_path, directory, wasm_path)?;
        let plugin_id = plugin.plugin_id().to_string();

        if self.plugins.insert(plugin_id.clone(), plugin).is_some() {
            return Err(invalid_data(format!("duplicate plugin id '{}'", plugin_id)));
        }

        Ok(())
    }

    fn build_plugin(
        &mut self,
        manifest: PluginManifest,
        manifest_path: std::path::PathBuf,
        directory: std::path::PathBuf,
        wasm_path: std::path::PathBuf,
    ) -> PluginResult<LoadedPlugin> {
        match manifest.runtime {
            PluginRuntimeKind::Component => {
                let component = self.cache.get_or_load(self.engine.as_ref(), &wasm_path)?;
                Ok(LoadedPlugin::component_plugin(
                    manifest,
                    manifest_path,
                    directory,
                    wasm_path,
                    component,
                ))
            }
            PluginRuntimeKind::Core => {
                let module = self
                    .cache
                    .get_or_load_module(self.engine.as_ref(), &wasm_path)?;
                Ok(LoadedPlugin::module_plugin(
                    manifest,
                    manifest_path,
                    directory,
                    wasm_path,
                    module,
                ))
            }
        }
    }
}

fn build_component_engine() -> PluginResult<Engine> {
    let mut config = WasmtimeConfig::new();
    config.wasm_component_model(true);
    config.consume_fuel(true);
    config.epoch_interruption(true);
    Engine::new(&config).map_err(Into::into)
}
