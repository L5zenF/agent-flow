use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::routing::{any, get, post};
use clap::{Args, Parser, Subcommand};
use proxy_tools::admin_api::{
    AdminState, get_config, get_plugins, put_config, reload_config, validate_config_handler,
};
use proxy_tools::config::load_config;
use proxy_tools::crypto::encrypt_header_value;
use proxy_tools::frontend::{panel_asset, panel_index};
use proxy_tools::gateway::{GatewayState, proxy_request};
use proxy_tools::wasm_plugins::load_plugin_registry;
use reqwest::Client;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Parser)]
#[command(name = "proxy-tools")]
#[command(about = "Generic LLM gateway with header injection rules")]
struct Cli {
    #[arg(long, default_value = "config/gateway.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    EncryptHeader(EncryptHeaderArgs),
}

#[derive(Debug, Args)]
struct EncryptHeaderArgs {
    #[arg(long)]
    value: String,
    #[arg(long)]
    secret_env: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into()),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    match cli.command {
        Some(Command::EncryptHeader(args)) => {
            println!(
                "{}",
                encrypt_header_value(&args.value, &args.secret_env)
                    .map_err(|error| format!("failed to encrypt header: {error}"))?
            );
            Ok(())
        }
        None => serve(cli).await,
    }
}

async fn serve(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config(&cli.config)?;
    let plugins_root = resolve_plugins_root(&cli.config)?;
    let plugin_registry = Arc::new(load_plugin_registry(&plugins_root)?);
    let gateway_addr: SocketAddr = config.listen.parse()?;
    let admin_addr: SocketAddr = config.admin_listen.parse()?;
    let shared_config = Arc::new(RwLock::new(config));

    let gateway_state = GatewayState {
        client: Client::builder().build()?,
        config: shared_config.clone(),
        plugin_registry: plugin_registry.clone(),
    };
    let admin_state = AdminState {
        config: shared_config.clone(),
        config_path: cli.config.clone(),
        plugin_registry: plugin_registry.clone(),
    };
    let gateway_app = Router::new()
        .route("/", any(proxy_request))
        .route("/{*rest}", any(proxy_request))
        .with_state(gateway_state);

    let admin_app = Router::new()
        .route("/admin/ui", get(panel_index))
        .route("/admin/ui/{*path}", get(panel_asset))
        .route("/admin/config", get(get_config).put(put_config))
        .route("/admin/plugins", get(get_plugins))
        .route("/admin/validate", post(validate_config_handler))
        .route("/admin/reload", post(reload_config))
        .with_state(admin_state);

    let gateway_listener = TcpListener::bind(gateway_addr).await?;
    let admin_listener = TcpListener::bind(admin_addr).await?;

    info!(listen = %gateway_addr, "gateway listening");
    info!(admin_listen = %admin_addr, "admin listening");
    info!(
        plugins_root = %plugins_root.display(),
        plugin_count = plugin_registry.len(),
        "wasm plugin registry loaded"
    );

    let gateway_server =
        axum::serve(gateway_listener, gateway_app).with_graceful_shutdown(shutdown_signal());
    let admin_server =
        axum::serve(admin_listener, admin_app).with_graceful_shutdown(shutdown_signal());

    tokio::try_join!(gateway_server, admin_server)?;
    Ok(())
}

fn resolve_plugins_root(
    config_path: &std::path::Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
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
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "resolved plugins path '{}' exists but is not a directory",
                    candidate.display()
                ),
            )));
        }
    }

    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!(
            "could not resolve plugins directory for config '{}'; checked: {}",
            canonical_config_path.display(),
            checked_locations.join(", ")
        ),
    )))
}

async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => info!("shutdown signal received"),
        Err(error) => warn!("failed to listen for shutdown signal: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_plugins_root;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be monotonic enough for tests")
            .as_nanos();
        dir.push(format!(
            "proxy-tools-main-{name}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp dir should be creatable");
        dir
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
    fn rejects_missing_plugins_directory() {
        let root = temp_dir("missing-plugins");
        let config_dir = root.join("config");
        let config_path = config_dir.join("gateway.toml");

        fs::create_dir_all(&config_dir).expect("config dir should be creatable");
        fs::write(&config_path, "listen = \"127.0.0.1:3000\"\n")
            .expect("config file should be writable");

        let error = resolve_plugins_root(&config_path)
            .expect_err("missing plugins root should be rejected");
        assert!(
            error
                .to_string()
                .contains("could not resolve plugins directory"),
            "unexpected error: {error}"
        );
    }
}
