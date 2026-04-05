# WASM Plugin Nodes Design

**Goal:** Add a `wasm_plugin` rule-graph node type so users can extend gateway behavior with custom WebAssembly logic while keeping workflow orchestration in the existing visual graph.

**Scope:** This design adds plugin discovery, runtime execution, config schema, validation, and editor support. It does not add a remote registry, plugin marketplace, or general-purpose user scripting environment.

## Why This Fits The Current Architecture

The existing rule graph is already a host-controlled interpreter implemented in `src/gateway.rs`. Node execution is serialized and mutates a bounded state surface:

- selected provider/model
- resolved path
- outgoing headers
- workflow context
- next node selection

This makes `wasm_plugin` a natural additional node type. The host remains responsible for graph traversal, validation, and final request forwarding.

## Design Principles

- Plugins extend a single node, not the whole workflow.
- Users still compose business flow by dragging graph nodes and edges.
- Plugin I/O must be explicit and versioned.
- The host must be able to bound CPU time, memory, and capabilities.
- The first version should prefer local files and deterministic execution.

## Runtime Choice

Use `Wasmtime` with the WebAssembly Component Model and WASI Preview 2.

Reasons:

- Rust and Wasmtime both support component-oriented host/plugin bindings.
- WIT gives a stable, typed ABI for plugin inputs and outputs.
- Wasmtime supports store-level limits, precompiled components, and host-controlled linking.

## Plugin Packaging

Plugins live in a local directory, for example:

```text
plugins/
  intent-classifier/
    plugin.toml
    plugin.wasm
```

`plugin.toml` stores:

- `id`
- `name`
- `version`
- `description`
- supported output ports
- optional default config schema hints

The sidecar manifest is chosen over self-describing wasm to keep editor discovery and validation simple.

## Rule Graph Model Changes

Add `wasm_plugin` to `RuleGraphNodeType`.

Add `WasmPluginNodeConfig` in backend and frontend types:

```toml
[rule_graph.nodes.wasm_plugin]
plugin_id = "intent-classifier"
timeout_ms = 20
fuel = 500000
max_memory_bytes = 16777216

[rule_graph.nodes.wasm_plugin.config]
prompt = "classify request intent"
default_intent = "chat"
```

Fields:

- `plugin_id`: plugin directory / manifest id
- `timeout_ms`: per-node execution budget
- `fuel`: optional Wasmtime fuel limit
- `max_memory_bytes`: per-instance memory cap
- `config`: arbitrary plugin-specific key/value config persisted in TOML

## Plugin ABI

Define the ABI with WIT in `wit/proxy-node-plugin.wit`.

Host passes:

- request method
- current path
- request headers
- workflow context
- selected provider id
- selected model id
- node config

Plugin returns:

- `context_patch`
- `header_ops`
- `path_rewrite`
- `next_port`
- `logs`

The plugin does not own graph traversal. It can only suggest the next output port. The host maps that port to an edge and continues normal graph execution.

## Execution Semantics

When `execute_rule_graph` reaches `wasm_plugin`:

1. Resolve the plugin from the in-memory registry.
2. Create a fresh Wasmtime `Store`.
3. Apply memory and fuel limits.
4. Instantiate the component with minimal WASI.
5. Call the plugin entrypoint with the current node input.
6. Apply returned patches to context / headers / path.
7. Follow the edge matching `next_port`, or the default linear edge if absent.

If plugin execution fails, return a graph execution error for the request. A future version may support explicit `error` ports, but the first version should keep failure semantics simple.

## Host Capability Boundary

The host capability model should be explicit and grant-based, not implicit.

First version should support three capability classes:

- `log`
- `fs`
- `network`

These capabilities are not globally enabled. A plugin receives them only when both of the following are true:

- the plugin manifest declares the capability requirement
- the graph node config explicitly grants that capability

Example manifest intent:

```toml
capabilities = ["fs", "network"]
```

Example node grant:

```toml
[rule_graph.nodes.wasm_plugin]
plugin_id = "remote-policy"
granted_capabilities = ["fs", "network"]
```

### Filesystem

Filesystem access should be directory-scoped, not host-global.

Node config should grant a list of readable and writable paths, for example:

```toml
read_dirs = ["plugins-data/common", "data/rules"]
write_dirs = ["plugins-data/runtime"]
```

The host should preopen only these directories for the plugin instance. No raw access to the repository root, home directory, or arbitrary absolute paths.

### Network

Network access should be outbound-only and host-filtered.

Node config should grant allowlisted destinations, for example:

```toml
allowed_hosts = ["api.example.com:443", "10.0.0.12:8080"]
```

The host should deny all network access not present in the allowlist. This preserves extensibility without turning plugin code into unrestricted remote execution.

### Non-goals

Do not expose by default:

- unrestricted filesystem access
- unrestricted outbound network access
- arbitrary environment variables

Environment variable access, clocks, and randomness should remain separately reviewed capabilities if they are ever added later.

## Validation Rules

Config validation should reject:

- missing `plugin_id`
- unknown plugin ids
- non-positive limits
- granted capabilities not declared by the plugin manifest
- `fs` grants without declared directories
- `network` grants without declared allowlisted hosts
- `next_port` edges that reference no valid outgoing handle
- duplicate plugin ids in manifests

Editor validation should surface:

- missing plugin selection
- plugin manifest load failure
- graph edge mismatch against declared plugin output ports

## Frontend Changes

In the editor:

- add `Wasm Plugin` to the node library
- render plugin-specific output handles from manifest ports
- add an inspector section for plugin selection, limits, and JSON/TOML-style config editing
- show manifest metadata to help users understand what the plugin does

The UI should treat plugins as first-class nodes, not as opaque blobs.

## Backend Changes

Add a new module such as `src/wasm_plugins.rs` responsible for:

- scanning `plugins/`
- reading manifests
- preloading / caching `Component`
- instantiating plugins per request
- translating WIT types to internal graph state patches

`src/config.rs` owns static config schema and validation.

`src/gateway.rs` owns runtime execution and state application.

## Risks And Mitigations

- ABI churn: version the WIT package and keep plugin output minimal.
- Debug complexity: log plugin id, node id, chosen port, and returned mutations.
- Runtime cost: preload components and create a fresh store per request.
- Unsafe capability growth: require manifest declaration plus per-node grant plus host-side allowlists.

## MVP

The first implementation should include:

- local plugin discovery
- one sample Rust plugin
- `wasm_plugin` node type
- WIT-defined typed ABI
- Wasmtime execution with limits
- explicit `fs` / `network` capability grants
- editor support for selecting a plugin and editing config

That is enough to prove the extensibility model without turning the gateway into a separate automation runtime.
