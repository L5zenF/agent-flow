use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use wasmtime::{Engine, Module, component::Component};

type PluginResult<T> = Result<T, Box<dyn Error>>;

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
