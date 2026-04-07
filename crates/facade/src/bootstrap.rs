use std::path::Path;

use application::{GatewaySummary, summarize_gateway_catalog, summarize_gateway_from_source};
use infrastructure::config_acl::{map_gateway_config, parse_raw_gateway_config};
use infrastructure::config_store::GatewayConfigFileSource;

use crate::error::FacadeError;

pub fn summarize_gateway_from_raw_toml(raw_toml: &str) -> Result<GatewaySummary, FacadeError> {
    let raw = parse_raw_gateway_config(raw_toml)?;
    let (catalog, workflows) = map_gateway_config(&raw)?;
    Ok(summarize_gateway_catalog(&catalog, &workflows)?)
}

pub fn summarize_gateway_from_path(path: &Path) -> Result<GatewaySummary, FacadeError> {
    let source = GatewayConfigFileSource::new(path);
    Ok(summarize_gateway_from_source(&source)?)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{summarize_gateway_from_path, summarize_gateway_from_raw_toml};

    #[test]
    fn summarizes_end_to_end_from_minimal_toml() {
        let summary = summarize_gateway_from_raw_toml(
            r#"
active_workflow_id = "chat-routing"

[[providers]]
id = "openai"
name = "OpenAI"

[[models]]
id = "gpt-4o"
name = "GPT-4o"
provider_id = "openai"

[[workflows]]
id = "chat-routing"
file = "chat-routing.toml"
"#,
        )
        .expect("raw toml should map and summarize");

        assert_eq!(summary.provider_count, 1);
        assert_eq!(summary.model_count, 1);
        assert_eq!(summary.workflow_count, 1);
        assert_eq!(summary.active_workflow_id, "chat-routing");
    }

    fn temp_file_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        path.push(format!("proxy-tools-facade-{name}-{stamp}.toml"));
        path
    }

    #[test]
    fn summarizes_from_config_file_path() {
        let config_path = temp_file_path("summary");
        std::fs::write(
            &config_path,
            r#"
active_workflow_id = "chat-routing"

[[providers]]
id = "openai"
name = "OpenAI"

[[models]]
id = "gpt-4o"
name = "GPT-4o"
provider_id = "openai"

[[workflows]]
id = "chat-routing"
file = "chat-routing.toml"
"#,
        )
        .expect("test config file should be writable");

        let summary = summarize_gateway_from_path(&config_path)
            .expect("config file should summarize through full stack");
        assert_eq!(summary.provider_count, 1);
        assert_eq!(summary.active_workflow_id, "chat-routing");

        let _ = std::fs::remove_file(config_path);
    }
}
