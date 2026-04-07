mod handler;
mod runtime;

pub use handler::{proxy_request, GatewayState};

#[cfg(test)]
pub(crate) use runtime::{
    plugin_workspace_root, resolve_plugin_network_policy, resolve_plugin_preopens,
    socket_addr_allowed, GatewayGraphExecutor, PluginNodeRuntime, WASMTIME_PLUGIN_RUNTIME,
};

#[cfg(test)]
mod tests;
