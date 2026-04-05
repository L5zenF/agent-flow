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

It may also include `default_config_schema_hints` when the plugin wants to describe the shape of its expected node config.

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
