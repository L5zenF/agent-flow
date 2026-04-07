use std::path::PathBuf;

use facade::summarize_gateway_from_path;

fn main() {
    let config_path = parse_config_path();
    match summarize_gateway_from_path(&config_path) {
        Ok(summary) => {
            println!("gateway summary");
            println!("config: {}", config_path.display());
            println!("providers: {}", summary.provider_count);
            println!("models: {}", summary.model_count);
            println!("workflows: {}", summary.workflow_count);
            println!("active_workflow_id: {}", summary.active_workflow_id);
        }
        Err(error) => {
            eprintln!(
                "failed to summarize gateway config '{}': {error}",
                config_path.display()
            );
            std::process::exit(1);
        }
    }
}

fn parse_config_path() -> PathBuf {
    let mut args = std::env::args().skip(1);
    let mut config_path = PathBuf::from("config/gateway.toml");

    while let Some(arg) = args.next() {
        if arg == "--config" {
            if let Some(value) = args.next() {
                config_path = PathBuf::from(value);
            }
        }
    }

    config_path
}
