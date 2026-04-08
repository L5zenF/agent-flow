use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

use serde::Deserialize;

use crate::plugin_registry::paths::invalid_data;

use super::model::{
    ManifestCapability, ManifestCategory, ManifestIcon, ManifestTone, PluginConfigSchema,
    PluginManifest, PluginManifestUi, PluginRuntimeKind,
};

type PluginResult<T> = Result<T, Box<dyn Error>>;

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

pub(crate) fn parse_plugin_manifest(path: &Path, raw: &str) -> PluginResult<PluginManifest> {
    ManifestParser::new(path).parse(raw)
}

struct ManifestParser<'a> {
    path: &'a Path,
}

impl<'a> ManifestParser<'a> {
    fn new(path: &'a Path) -> Self {
        Self { path }
    }

    fn parse(&self, raw: &str) -> PluginResult<PluginManifest> {
        let raw_manifest: RawPluginManifest = toml::from_str(raw).map_err(|error| {
            invalid_data(format!(
                "failed to parse plugin manifest '{}': {error}",
                self.path.display()
            ))
        })?;

        let capabilities = raw_manifest
            .capabilities
            .into_iter()
            .map(|capability| self.parse_manifest_capability(&capability))
            .collect::<PluginResult<Vec<_>>>()?;
        let supported_output_ports =
            self.validate_supported_output_ports(raw_manifest.supported_output_ports)?;
        let ui = self.parse_manifest_ui(raw_manifest.ui)?;

        Ok(PluginManifest {
            id: raw_manifest.id,
            name: raw_manifest.name,
            version: raw_manifest.version,
            description: raw_manifest.description,
            runtime: self.parse_plugin_runtime(raw_manifest.runtime.as_deref())?,
            supported_output_ports,
            capabilities,
            default_config_schema_hints: raw_manifest.default_config_schema_hints,
            config_schema: raw_manifest.config_schema,
            ui,
        })
    }

    fn parse_manifest_capability(&self, capability: &str) -> PluginResult<ManifestCapability> {
        match capability {
            "log" => Ok(ManifestCapability::Log),
            "fs" => Ok(ManifestCapability::Fs),
            "network" => Ok(ManifestCapability::Network),
            other => Err(invalid_data(format!(
                "plugin manifest '{}' declares unsupported capability '{}' (supported: log, fs, network)",
                self.path.display(),
                other
            ))),
        }
    }

    fn parse_plugin_runtime(&self, runtime: Option<&str>) -> PluginResult<PluginRuntimeKind> {
        match runtime.map(str::trim).filter(|value| !value.is_empty()) {
            None => Ok(PluginRuntimeKind::Component),
            Some("component") => Ok(PluginRuntimeKind::Component),
            Some("core") => Ok(PluginRuntimeKind::Core),
            Some(other) => Err(invalid_data(format!(
                "plugin manifest '{}' declares unsupported runtime '{}' (supported: component, core)",
                self.path.display(),
                other
            ))),
        }
    }

    fn parse_manifest_ui(&self, raw_ui: RawPluginManifestUi) -> PluginResult<PluginManifestUi> {
        let icon = raw_ui
            .icon
            .as_deref()
            .map(|value| self.parse_manifest_icon(value))
            .transpose()?;
        let category = raw_ui
            .category
            .as_deref()
            .map(|value| self.parse_manifest_category(value))
            .transpose()?;
        let tone = raw_ui
            .tone
            .as_deref()
            .map(|value| self.parse_manifest_tone(value))
            .transpose()?;
        let tags = raw_ui
            .tags
            .into_iter()
            .map(|tag| self.normalize_manifest_tag(tag))
            .collect::<PluginResult<Vec<_>>>()?;

        Ok(PluginManifestUi {
            icon,
            category,
            tone,
            order: raw_ui.order,
            tags,
        })
    }

    fn parse_manifest_icon(&self, value: &str) -> PluginResult<ManifestIcon> {
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
                self.path.display(),
                other
            ))),
        }
    }

    fn parse_manifest_category(&self, value: &str) -> PluginResult<ManifestCategory> {
        match value {
            "control" => Ok(ManifestCategory::Control),
            "transform" => Ok(ManifestCategory::Transform),
            "routing" => Ok(ManifestCategory::Routing),
            "policy" => Ok(ManifestCategory::Policy),
            "utility" => Ok(ManifestCategory::Utility),
            other => Err(invalid_data(format!(
                "plugin manifest '{}' declares unsupported ui.category '{}' (supported: control, transform, routing, policy, utility)",
                self.path.display(),
                other
            ))),
        }
    }

    fn parse_manifest_tone(&self, value: &str) -> PluginResult<ManifestTone> {
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
                self.path.display(),
                other
            ))),
        }
    }

    fn normalize_manifest_tag(&self, tag: String) -> PluginResult<String> {
        let normalized = tag.trim();
        if normalized.is_empty() {
            return Err(invalid_data(format!(
                "plugin manifest '{}' contains an empty ui.tags entry",
                self.path.display()
            )));
        }
        Ok(normalized.to_owned())
    }

    fn validate_supported_output_ports(&self, ports: Vec<String>) -> PluginResult<Vec<String>> {
        let mut seen = BTreeSet::new();
        let mut validated = Vec::with_capacity(ports.len());

        for port in ports {
            let normalized = port.trim();
            if normalized.is_empty() {
                return Err(invalid_data(format!(
                    "plugin manifest '{}' contains an empty supported_output_ports entry",
                    self.path.display()
                )));
            }
            if !seen.insert(normalized.to_owned()) {
                return Err(invalid_data(format!(
                    "plugin manifest '{}' declares duplicate supported_output_port '{}'",
                    self.path.display(),
                    normalized
                )));
            }
            validated.push(normalized.to_owned());
        }

        Ok(validated)
    }
}
