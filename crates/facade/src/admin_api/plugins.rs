use axum::Json;
use axum::extract::State;

use crate::admin_api::support::{
    manifest_capability_name, manifest_category_name, manifest_icon_name, manifest_tone_name,
};
use crate::admin_api::types::{AdminState, PluginManifestSummary, PluginManifestUiSummary};

pub async fn get_plugins(State(state): State<AdminState>) -> impl axum::response::IntoResponse {
    let plugins = state
        .plugin_registry
        .plugins()
        .map(|plugin| PluginManifestSummary {
            id: plugin.manifest().id.clone(),
            name: plugin.manifest().name.clone(),
            version: plugin.manifest().version.clone(),
            description: plugin.manifest().description.clone(),
            supported_output_ports: plugin.manifest().supported_output_ports.clone(),
            capabilities: plugin
                .manifest()
                .capabilities
                .iter()
                .map(manifest_capability_name)
                .map(str::to_string)
                .collect(),
            default_config_schema_hints: plugin.manifest().default_config_schema_hints.clone(),
            config_schema: plugin.manifest().config_schema.clone(),
            ui: PluginManifestUiSummary {
                icon: plugin
                    .manifest()
                    .ui
                    .icon
                    .as_ref()
                    .map(manifest_icon_name)
                    .map(str::to_string),
                category: plugin
                    .manifest()
                    .ui
                    .category
                    .as_ref()
                    .map(manifest_category_name)
                    .map(str::to_string),
                tone: plugin
                    .manifest()
                    .ui
                    .tone
                    .as_ref()
                    .map(manifest_tone_name)
                    .map(str::to_string),
                order: plugin.manifest().ui.order,
                tags: plugin.manifest().ui.tags.clone(),
            },
        })
        .collect::<Vec<_>>();

    Json(plugins)
}
