use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::http::HeaderMap;
use infrastructure::plugin_registry::LoadedPlugin;
use infrastructure::plugin_runtime_contract::RuntimeRequestHeader;
use wasmtime_wasi::sockets::SocketAddrUse;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtx, WasiCtxBuilder};

use super::{PluginNetworkPolicy, PluginPreopenDir};
use crate::config::{WasmCapability, WasmPluginNodeConfig};

pub(crate) fn build_plugin_wasi_ctx(
    plugin: &LoadedPlugin,
    node_config: &WasmPluginNodeConfig,
) -> Result<WasiCtx, String> {
    let mut builder = WasiCtxBuilder::new();
    builder.allow_blocking_current_thread(true);
    builder.allow_tcp(false);
    builder.allow_udp(false);
    builder.allow_ip_name_lookup(false);

    for preopen in resolve_plugin_preopens(plugin, node_config)? {
        builder
            .preopened_dir(
                &preopen.host_path,
                preopen.guest_path.as_str(),
                preopen.dir_perms,
                preopen.file_perms,
            )
            .map_err(|error| {
                format!(
                    "failed to preopen '{}' for plugin '{}': {error}",
                    preopen.host_path.display(),
                    plugin.plugin_id()
                )
            })?;
    }

    if let Some(policy) = resolve_plugin_network_policy(node_config)? {
        let allowed_addrs = policy.allowed_addrs.clone();
        builder.allow_tcp(true);
        builder.allow_udp(true);
        builder.allow_ip_name_lookup(policy.allow_ip_name_lookup);
        builder.socket_addr_check(move |addr, reason| {
            let allowed_addrs = allowed_addrs.clone();
            Box::pin(async move { socket_addr_allowed(&allowed_addrs, addr, reason) })
        });
    }

    Ok(builder.build())
}

pub(crate) fn resolve_plugin_preopens(
    plugin: &LoadedPlugin,
    node_config: &WasmPluginNodeConfig,
) -> Result<Vec<PluginPreopenDir>, String> {
    let root = plugin_workspace_root(plugin);
    let mut preopens = BTreeMap::<String, PluginPreopenDir>::new();

    for relative in &node_config.read_dirs {
        upsert_preopen_dir(
            &mut preopens,
            &root,
            relative,
            DirPerms::READ,
            FilePerms::READ,
        );
    }

    for relative in &node_config.write_dirs {
        upsert_preopen_dir(
            &mut preopens,
            &root,
            relative,
            DirPerms::READ | DirPerms::MUTATE,
            FilePerms::READ | FilePerms::WRITE,
        );
    }

    Ok(preopens.into_values().collect())
}

fn upsert_preopen_dir(
    preopens: &mut BTreeMap<String, PluginPreopenDir>,
    root: &Path,
    relative: &str,
    dir_perms: DirPerms,
    file_perms: FilePerms,
) {
    let guest_path = guest_preopen_path(relative);
    let host_path = root.join(relative);

    preopens
        .entry(guest_path.clone())
        .and_modify(|existing| {
            existing.dir_perms |= dir_perms;
            existing.file_perms |= file_perms;
        })
        .or_insert_with(|| PluginPreopenDir {
            host_path,
            guest_path,
            dir_perms,
            file_perms,
        });
}

fn guest_preopen_path(relative: &str) -> String {
    let trimmed = relative.trim_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        format!("/{trimmed}")
    }
}

pub(crate) fn plugin_workspace_root(plugin: &LoadedPlugin) -> PathBuf {
    plugin
        .directory()
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| plugin.directory().to_path_buf())
}

pub(crate) fn resolve_plugin_network_policy(
    node_config: &WasmPluginNodeConfig,
) -> Result<Option<PluginNetworkPolicy>, String> {
    if !node_config
        .granted_capabilities
        .iter()
        .any(|capability| matches!(capability, WasmCapability::Network))
    {
        return Ok(None);
    }

    let mut allowed_addrs = HashSet::new();
    let mut allow_ip_name_lookup = false;

    for host in &node_config.allowed_hosts {
        if host.parse::<SocketAddr>().is_err() {
            allow_ip_name_lookup = true;
        }

        let resolved = host
            .to_socket_addrs()
            .map_err(|error| format!("failed to resolve allowlisted host '{host}': {error}"))?;
        let mut resolved_any = false;
        for addr in resolved {
            allowed_addrs.insert(addr);
            resolved_any = true;
        }

        if !resolved_any {
            return Err(format!(
                "allowlisted host '{host}' resolved to no socket addresses"
            ));
        }
    }

    Ok(Some(PluginNetworkPolicy {
        allowed_addrs: Arc::new(allowed_addrs),
        allow_ip_name_lookup,
    }))
}

pub(crate) fn socket_addr_allowed(
    allowed_addrs: &HashSet<SocketAddr>,
    addr: SocketAddr,
    reason: SocketAddrUse,
) -> bool {
    match reason {
        SocketAddrUse::TcpBind | SocketAddrUse::UdpBind => false,
        SocketAddrUse::TcpConnect
        | SocketAddrUse::UdpConnect
        | SocketAddrUse::UdpOutgoingDatagram => allowed_addrs.contains(&addr),
    }
}

pub(crate) fn current_request_headers(
    incoming_headers: &HeaderMap,
    outgoing_headers: &HashMap<String, Vec<String>>,
) -> Vec<RuntimeRequestHeader> {
    let mut merged = incoming_headers
        .iter()
        .filter_map(|(name, value)| {
            value.to_str().ok().map(|value| RuntimeRequestHeader {
                name: name.as_str().to_string(),
                value: value.to_string(),
            })
        })
        .collect::<Vec<_>>();

    for (name, values) in outgoing_headers {
        merged.retain(|header| !header.name.eq_ignore_ascii_case(name));
        for value in values {
            merged.push(RuntimeRequestHeader {
                name: name.clone(),
                value: value.clone(),
            });
        }
    }

    merged
}
