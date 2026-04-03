# Canvas-First Admin Panel Design

**Date**: 2026-04-03

## Goal

Refactor the admin UI into a canvas-first flow editor:

- Keep a thin top bar
- Make the rule graph canvas the dominant surface
- Move provider/model management into a centered settings modal
- Keep load/save as icon actions in the top-right area
- Make validation automatic and non-intrusive

## User Experience

### Primary Layout

The default screen should feel like a workflow editor, not a config dashboard.

- A thin top bar remains visible
- The rest of the page is primarily the React Flow canvas
- The left node toolbox stays compact and floating
- The right property panel remains available for the selected node
- Global tabs and list-style management views are removed from the primary screen

### Top Bar

The top bar should be visually light and narrow.

- Left side: product title / environment identity
- Right side: icon-only controls
  - settings
  - load config
  - save config
- Validation status is shown as a compact indicator near the canvas, not as a full button

### Settings Modal

Settings opens as a centered modal.

- It contains provider management
- It contains model management
- It may also include small global config fields if needed:
  - `listen`
  - `admin_listen`
  - `default_secret_env`
- It should not feel like a separate admin page inside the modal
- It should be compact, scrollable, and structured in sections

### Validation

Validation runs automatically from local graph/config state.

- No dedicated validate button in the top-right toolbar
- If validation issues exist, show a red exclamation indicator at the top-right of the canvas area
- If no validation issues exist, either show nothing or a very subtle valid state
- Node-level issues still remain visible on nodes and in the inspector

## Architecture

### App-Level State

The app remains single-page, but navigation complexity is removed.

- `App.tsx` becomes a shell for:
  - top bar
  - canvas-first editor surface
  - settings modal
- provider/model editors move out of the main tabbed layout
- the active config remains the source of truth for save/load

### Rule Graph Editor

`RuleGraphEditor` becomes the main screen primitive.

- It owns the large canvas surface
- It exposes automatic validation state upward or renders top-right validation state internally
- It keeps the compact floating toolbox
- It keeps the node inspector for detailed node editing

### Settings Modal Ownership

The settings modal should be owned by `App.tsx` so toolbar actions can open/close it directly.

- `App.tsx` controls modal visibility
- Existing provider/model editing controls are repurposed into modal sections
- The modal edits the same shared `config` state

## Components

### Keep

- `RuleGraphEditor`
- existing form controls
- provider/model editing logic where reusable

### Replace / Restructure

- remove main tab switcher from the primary layout
- remove list-first resource screens from the default surface
- replace header action row with icon toolbar
- replace the current provider-first page framing with editor framing

### Add

- settings icon button
- load icon button
- save icon button
- centered settings modal
- compact canvas validation badge

## Data Flow

### Save / Load

- `Load` fetches config from `/admin/config` and repaints the canvas + modal-backed state
- `Save` flushes pending editor state, including graph positions and field drafts, then persists to `/admin/config`
- The current fix for rule graph flushing remains part of the save path

### Modal Editing

- Settings modal edits `config.providers`, `config.models`, and optional global fields directly
- Changes are reflected in graph dropdowns immediately

### Validation

- Validation is derived from in-memory config + graph
- The red indicator reflects current local issues before save
- Save still depends on server-side validation as final authority

## Error Handling

- Save/load failures continue to surface in the top status area or a compact top-bar status region
- Validation issues stay local and visual
- Modal edits should not silently discard state on close

## Testing

- Verify top bar only shows the compact icon actions
- Verify default landing view is canvas-first with no tab navigation
- Verify settings modal opens centered and edits providers/models successfully
- Verify save/load still persist rule graph positions and node config
- Verify validation indicator turns red when graph/global issues exist
- Verify canvas remains large on desktop and usable on smaller screens

## Scope Boundaries

This change is a UI restructuring, not a backend model redesign.

- No backend config format changes required
- No rule execution changes required
- No new admin API endpoints required
- No generic workflow platform behavior added

## Recommendation

Implement this as a focused frontend refactor:

- keep the existing config model
- keep the existing save/load API contract
- make the canvas the default information hierarchy
- move non-graph config into a centered settings modal

That achieves the intended “流程编排优先” feel without reopening backend scope.
