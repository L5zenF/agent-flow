# Inline Wasm Node Config Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render all business-facing wasm node configuration directly on each node while keeping permissions and runtime limits inside the modal.

**Architecture:** Reuse plugin `config_schema` as a generic inline form contract inside `RuleCanvasNode`. Keep `WasmConfigModal` as an operational settings surface for capabilities, filesystem/network scope, and runtime guardrails only.

**Tech Stack:** React, TypeScript, React Flow, existing wasm plugin manifest/admin API pipeline.

---

## File Map

**Create:**
- `docs/superpowers/specs/2026-04-07-inline-wasm-node-config-design.md`
- `docs/superpowers/plans/2026-04-07-inline-wasm-node-config.md`

**Modify:**
- `web/src/components/rule-graph-editor.tsx`

**Primary tests:**
- `cd web && npm run build`
- `cargo test`

### Task 1: Move Schema Rendering Into The Node Body

**Files:**
- Modify: `web/src/components/rule-graph-editor.tsx`

- [ ] Reuse the existing schema field helpers for inline node rendering.
- [ ] Add a generic inline wasm business config section inside `RuleCanvasNode`.
- [ ] Render `text`, `textarea`, `select`, and `boolean` fields directly on the node body.
- [ ] Wire field updates back through `updateWasmRuntimeConfig(...)`.

### Task 2: Reduce The Modal To Operational Settings

**Files:**
- Modify: `web/src/components/rule-graph-editor.tsx`

- [ ] Remove schema-driven business fields from `WasmConfigModal`.
- [ ] Keep plugin selection fallback, outputs, permissions, and runtime limits.
- [ ] Keep raw JSON editing only as fallback for plugins without `config_schema`.

### Task 3: Verify Node UX For Select Model And Route Provider

**Files:**
- Modify: `web/src/components/rule-graph-editor.tsx`

- [ ] Confirm `select-model` renders provider/model selects on the node.
- [ ] Confirm `route-provider` renders provider select on the node.
- [ ] Confirm the modal is no longer required for these business edits.

### Task 4: Verification

**Files:**
- No code changes required

- [ ] Run `cd web && npm run build`
- [ ] Run `cargo test`
- [ ] Review the canvas node layout for duplicated business fields or modal-only remnants
