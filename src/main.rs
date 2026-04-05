use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::{any, get, post};
use axum::{Extension, Router};
use clap::{Args, Parser, Subcommand};
use proxy_tools::admin_api::{
    AdminState, get_config, put_config, reload_config, validate_config_handler,
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
    let plugin_registry = Arc::new(load_plugin_registry(Path::new("plugins"))?);
    let gateway_addr: SocketAddr = config.listen.parse()?;
    let admin_addr: SocketAddr = config.admin_listen.parse()?;
    let shared_config = Arc::new(RwLock::new(config));

    let gateway_state = GatewayState {
        client: Client::builder().build()?,
        config: shared_config.clone(),
    };
    let admin_state = AdminState {
        config: shared_config.clone(),
        config_path: cli.config.clone(),
    };
    let gateway_app = Router::new()
        .route("/", any(proxy_request))
        .route("/{*rest}", any(proxy_request))
        .with_state(gateway_state)
        .layer(Extension(plugin_registry.clone()));

    let admin_app = Router::new()
        .route("/admin/ui", get(panel_index))
        .route("/admin/ui/{*path}", get(panel_asset))
        .route("/admin/config", get(get_config).put(put_config))
        .route("/admin/validate", post(validate_config_handler))
        .route("/admin/reload", post(reload_config))
        .with_state(admin_state)
        .layer(Extension(plugin_registry));

    let gateway_listener = TcpListener::bind(gateway_addr).await?;
    let admin_listener = TcpListener::bind(admin_addr).await?;

    info!(listen = %gateway_addr, "gateway listening");
    info!(admin_listen = %admin_addr, "admin listening");

    let gateway_server =
        axum::serve(gateway_listener, gateway_app).with_graceful_shutdown(shutdown_signal());
    let admin_server =
        axum::serve(admin_listener, admin_app).with_graceful_shutdown(shutdown_signal());

    tokio::try_join!(gateway_server, admin_server)?;
    Ok(())
}

async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => info!("shutdown signal received"),
        Err(error) => warn!("failed to listen for shutdown signal: {error}"),
    }
}
