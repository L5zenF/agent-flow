# Inline Wasm Node Config Design

**Date**: 2026-04-07

## Goal

Move business-facing wasm node configuration out of the modal and onto the node body itself.

- Keep `Permissions` and `Runtime Limits` in the modal
- Render all business fields directly on the canvas node
- Reuse `config_schema` as the single generic rendering contract
- Avoid per-plugin frontend component code

## User Experience

### Primary Editing Model

The node body is the primary editing surface for business logic.

- Users should edit provider, model, path, headers, expressions, and other business fields directly on the node
- Opening a modal should no longer be required for normal workflow authoring
- The node action button remains available for lower-frequency operational settings

### Node Content Layout

Each wasm-backed business node keeps a compact three-part structure:

- Header: title, description, actions
- Body: schema-rendered business fields
- Footer: output handles and validation

### Modal Scope

The wasm modal becomes operational-only.

- `Permissions`
- `Runtime Limits`

It should no longer be the primary place to edit plugin business config.

## Schema Rendering Rules

### Generic Renderer

Render `config_schema.fields` directly inside the node body.

Supported field types in this iteration:

- `text`
- `textarea`
- `select`
- `boolean`

Supported dynamic data sources in this iteration:

- `providers`
- `models`

Dependency support:

- `depends_on`

### Fallback Behavior

- If a plugin declares `config_schema`, render those fields inline
- If a plugin has no `config_schema`, fall back to raw JSON editing only in the modal

## Node-Specific Behavior

### Match

`Match` remains a special node because its branches and fallback belong to graph structure, not plugin config.

- Keep branch editing inline on the node
- Keep plugin selection internal/defaulted
- Do not move branch editing into the modal

### Wasm Business Nodes

These should be edited inline via schema:

- `Condition`
- `Select Model`
- `Route Provider`
- `Rewrite Path`
- `Set Header`
- `Log`
- future wasm-backed business nodes

## Architecture

### Frontend

`RuleCanvasNode` should own inline business-field rendering for wasm-backed nodes.

- Read `pluginManifest.config_schema`
- Resolve select options from already-loaded provider/model data
- Write updates directly into `node.wasm_plugin.config`

### Modal

`WasmConfigModal` becomes a reduced operational editor.

- Remove inline business `Plugin Config` controls from the modal when schema is present
- Keep runtime and capability editing there

## Error Handling

- Missing required fields remain visible through existing validation
- Invalid provider/model selections should still show empty or stale values rather than silently reset
- Option filtering for `depends_on` must not destroy existing config when parent fields change

## Testing

- Verify `Select Model` renders provider/model selects directly on the node
- Verify `Route Provider` renders provider select directly on the node
- Verify node edits update graph state without opening the modal
- Verify the modal still edits permissions and runtime limits
- Verify frontend build passes

## Scope Boundaries

- No backend execution-model changes
- No new schema field types beyond the first four
- No generic dynamic option API beyond current provider/model data already present in the editor
