# Multi-Workflow Gallery Design

## Summary

The admin UI should stop assuming a single `rule_graph` embedded in `gateway.toml`.
Instead, the gateway should manage a directory of workflow files and expose a gallery-first UI that lets users browse, create, edit, and activate one workflow at a time.

The first version should optimize for clarity over flexibility:

- global gateway settings remain in `gateway.toml`
- workflow content moves into `workflows/*.toml`
- exactly one workflow is active at runtime
- the admin UI opens on a workflow gallery rather than dropping directly into the canvas

## Goals

- support multiple workflows without making one config file unmaintainable
- let users visually choose the active workflow from a gallery
- keep runtime behavior explicit: only the active workflow executes
- preserve the existing graph editor with minimal conceptual change once a workflow is opened

## Non-Goals

- workflow version history
- concurrent execution of multiple workflows
- workflow inheritance or composition
- remote workflow storage

## Configuration Model

`gateway.toml` remains the source of truth for global settings, providers, and models.
It gains workflow management metadata:

```toml
workflows_dir = "workflows"
active_workflow_id = "chat-routing"

[[workflows]]
id = "chat-routing"
name = "Chat Routing"
file = "chat-routing.toml"
description = "Main chat entry flow"
```

Each workflow file stores one graph only:

```toml
[workflow]
id = "chat-routing"
version = 1
start_node_id = "start"

[[workflow.nodes]]
id = "start"
type = "start"
```

The workflow file should contain only graph content plus workflow-local metadata needed for editing.
Provider and model definitions stay global in `gateway.toml`.

## Runtime Behavior

On startup and reload:

- load `gateway.toml`
- resolve `workflows_dir`
- load the indexed workflow files
- validate that `active_workflow_id` exists in the workflow index
- execute only the active workflow for live traffic

If the active workflow file is missing or invalid, startup should fail closed with a clear error.
Inactive workflow files may also fail validation, but the admin UI should surface those as gallery errors rather than silently ignoring them.

## Admin API

The admin API should separate gateway settings from workflow content:

- `GET /admin/config`
  Returns global gateway config plus workflow index metadata.
- `GET /admin/workflows`
  Returns gallery summaries with `id`, `name`, `description`, `file`, `is_active`, `node_count`, and `edge_count`.
- `GET /admin/workflows/:id`
  Returns one workflow document for editing.
- `POST /admin/workflows`
  Creates a new indexed workflow and its file.
- `PUT /admin/workflows/:id`
  Saves one workflow file.
- `POST /admin/workflows/:id/activate`
  Updates `active_workflow_id`.

## UI Flow

The admin UI should open on a gallery view:

- cards show workflow name, description, active badge, and a lightweight graph thumbnail
- primary actions are `Open`, `Set Active`, and `New Workflow`
- opening a workflow transitions into the existing canvas editor scoped to that workflow
- the editor should show the workflow name and activation state in its top bar

The settings modal continues to edit global gateway settings, providers, and models.
Workflow management moves out of that modal into the gallery.

## Migration

If legacy config contains a single `rule_graph`, the system should migrate it into:

- `workflows/default.toml`
- a workflow index entry with `id = "default"`
- `active_workflow_id = "default"`

The first save after migration should write the new structure back to disk.

## Testing

Add tests for:

- parsing and saving the new workflow index structure
- loading workflow files from `workflows_dir`
- rejecting missing active workflow references
- legacy single-graph migration
- admin API workflow CRUD and activation flows
- gallery rendering and switching between gallery and editor views
