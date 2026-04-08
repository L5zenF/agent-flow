mod cache;
mod loader;
mod manifest;
mod paths;
mod registry;
#[cfg(test)]
mod tests;

pub use cache::ComponentCache;
pub use loader::load_plugin_registry;
pub use manifest::{
    ManifestCapability, ManifestCategory, ManifestIcon, ManifestTone, PluginConfigField,
    PluginConfigFieldDataSource, PluginConfigFieldType, PluginConfigSchema, PluginManifest,
    PluginManifestUi, PluginRuntimeKind,
};
pub use paths::resolve_plugins_root;
pub use registry::{LoadedPlugin, PluginRegistry};
