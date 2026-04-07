mod context;
mod graph;
mod resolve;
mod route_match;

pub use context::{
    inject_runtime_context,
};
pub use graph::{execute_rule_graph, GraphNodeExecutor};
pub use resolve::resolve_request;
pub use route_match::{format_header_names, resolve_route};

#[cfg(test)]
pub(crate) use context::{
    next_condition_edge, next_linear_edge, resolve_provider_header_for_graph,
    sync_selected_targets_from_context,
};

#[cfg(test)]
pub(crate) use resolve::RequestResolution;

#[cfg(test)]
pub(crate) use route_match::evaluate_router_clause;
