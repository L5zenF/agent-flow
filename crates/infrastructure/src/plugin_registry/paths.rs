use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type PluginResult<T> = Result<T, Box<dyn Error>>;

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

pub(crate) fn resolve_plugin_wasm_path(directory: &Path) -> PluginResult<PathBuf> {
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

pub(crate) fn invalid_data(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message.into(),
    ))
}
