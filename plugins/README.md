# Plugin Layout

Local wasm plugins live under `plugins/<plugin-id>/`.

Each plugin directory should contain:

- `plugin.toml` for the package manifest and metadata.
- `plugin.wasm` for the compiled WebAssembly component that implements the `proxy-node-plugin` world.

`plugin.toml` must declare these required fields:

- `id`
- `name`
- `version`
- `description`
- `supported_output_ports`

It may also include `default_config_schema_hints` when the plugin wants to describe the shape of its expected node config. Treat that field as a JSON document rather than a free-form string so the shape can evolve without freezing the ABI.

Node-specific plugin config should likewise be carried as a JSON document, not flattened into string key/value pairs. That keeps the first-pass ABI simple while leaving room for structured config and capability data later.

The manifest should also declare the capabilities the plugin expects to use. Keep those declarations narrow and explicit so the host can validate them before execution.

The node that loads the plugin also needs to grant the capabilities it is willing to expose. Those node-level grants should be treated as the runtime allowlist for the manifest declarations.

Recommended shape:

```text
plugins/
  my-plugin/
    plugin.toml
    plugin.wasm
```

In practice:

- Manifest capability declarations describe what the plugin needs.
- Node-level capability grants describe what the host actually allows for that plugin instance.
- Execution should fail closed when a declared capability is missing from the node grant set.
- Keep `fs` access narrow by using relative paths inside granted read/write directories.
- Keep `network` access narrow by limiting plugins to explicit hosts that return simple JSON payloads.

Example node config:

```toml
[rule_graph.nodes.wasm_plugin]
plugin_id = "match-evaluator"
timeout_ms = 20
fuel = 500000
max_memory_bytes = 16777216
granted_capabilities = ["fs", "network"]
read_dirs = ["plugins-data/common"]
write_dirs = ["plugins-data/runtime"]
allowed_hosts = ["api.example.com:443"]

[rule_graph.nodes.wasm_plugin.config]
prompt = "classify request intent"
default_intent = "chat"
```

The admin UI reads plugin metadata from `/admin/plugins` and uses these manifest fields:

- `id`
- `name`
- `version`
- `description`
- `supported_output_ports`
- `capabilities`
- `default_config_schema_hints`

Included sample plugins:

- `match-evaluator` for branch-by-branch request matching.

For file-based plugins, prefer guest paths that match the granted directory mount.
