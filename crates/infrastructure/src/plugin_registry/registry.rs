use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use wasmtime::{Engine, Module, component::Component};

use super::cache::ComponentCache;
use super::manifest::{PluginManifest, PluginRuntimeKind};

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
    pub(crate) fn component_plugin(
        manifest: PluginManifest,
        manifest_path: PathBuf,
        directory: PathBuf,
        wasm_path: PathBuf,
        component: Arc<Component>,
    ) -> Self {
        Self {
            manifest,
            manifest_path,
            directory,
            wasm_path,
            artifact: LoadedPluginArtifact::Component(component),
        }
    }

    pub(crate) fn module_plugin(
        manifest: PluginManifest,
        manifest_path: PathBuf,
        directory: PathBuf,
        wasm_path: PathBuf,
        module: Arc<Module>,
    ) -> Self {
        Self {
            manifest,
            manifest_path,
            directory,
            wasm_path,
            artifact: LoadedPluginArtifact::Module(module),
        }
    }

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
    pub(crate) fn empty(plugins_root: PathBuf, engine: Arc<Engine>) -> Self {
        Self {
            plugins_root,
            engine,
            component_cache: ComponentCache::new(),
            plugins: BTreeMap::new(),
        }
    }

    pub(crate) fn loaded(
        plugins_root: PathBuf,
        engine: Arc<Engine>,
        component_cache: ComponentCache,
        plugins: BTreeMap<String, LoadedPlugin>,
    ) -> Self {
        Self {
            plugins_root,
            engine,
            component_cache,
            plugins,
        }
    }

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
