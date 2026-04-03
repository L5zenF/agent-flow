# Canvas-First Admin Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the admin UI so the default experience is a large rule-graph canvas with a thin top bar, icon-only actions, a centered settings modal for providers/models, and automatic validation indicators.

**Architecture:** Collapse the current tabbed admin shell into a single canvas-first layout in `web/src/App.tsx`, keep `RuleGraphEditor` as the main interaction surface, and move provider/model/global config editing into a modal owned by `App.tsx`. Reuse existing config state and save/load API flow, including the pending-editor flush path, while simplifying the page hierarchy and exposing validation through a compact canvas badge instead of a dedicated button.

**Tech Stack:** React 19, TypeScript, Vite, React Flow, lucide-react, existing local UI primitives

---

### Task 1: Replace The Tabbed App Shell With A Canvas-First Frame

**Files:**
- Modify: `web/src/App.tsx`
- Verify: `web/src/App.tsx`

- [ ] **Step 1: Identify the old shell sections to remove**

Read and map the parts of `web/src/App.tsx` that will no longer be primary-screen UI:

- `tabs`
- `TabKey`
- top-level conditional rendering for `providers/models/routes/rules/raw`
- provider editing dialog state that only exists to support tab navigation

Expected outcome: a simplified app shell with one primary content region for the graph editor.

- [ ] **Step 2: Remove the tab switcher and list-first primary views**

Rewrite the top-level render so the main body always renders the graph editor instead of switching on `tab`.

Target structure:

```tsx
return (
  <div className="min-h-screen bg-zinc-50 text-zinc-900">
    <div className="flex min-h-screen flex-col">
      <TopBar ... />
      <main className="flex-1 px-3 pb-3 pt-2 lg:px-4">
        <RuleGraphEditor config={config} setConfig={setConfig} />
      </main>
      <SettingsModal ... />
    </div>
  </div>
);
```

- [ ] **Step 3: Keep save/load plumbing but move it into icon-toolbar actions**

Preserve:

- `flushPendingEditorState`
- `load`
- `save`
- `reload`

Remove:

- dedicated `validate` button
- tab-driven layout dependencies

- [ ] **Step 4: Run a focused build check**

Run: `npm run build`

Expected: build may fail because modal/top-bar helper components are not added yet, but there should be no unrelated type regressions. Note the failing symbols before continuing.

- [ ] **Step 5: Commit the shell rewrite**

```bash
git add web/src/App.tsx
git commit -m "refactor: make admin shell canvas-first"
```

### Task 2: Add A Thin Top Bar With Icon Actions

**Files:**
- Modify: `web/src/App.tsx`
- Modify: `web/src/styles.css`
- Verify: `web/src/App.tsx`

- [ ] **Step 1: Add the top bar action model**

Define a compact top bar in `App.tsx` with:

- left title block
- right icon-only buttons for:
  - settings
  - load
  - save

Use lucide icons already present or add `Settings2`/`SlidersHorizontal` if needed.

- [ ] **Step 2: Replace text-heavy action buttons with icon buttons**

Implement action button shape similar to:

```tsx
<button
  type="button"
  onClick={save}
  className="inline-flex h-10 w-10 items-center justify-center rounded-full border border-zinc-200 bg-white text-zinc-700 transition hover:border-zinc-900 hover:text-zinc-900 disabled:opacity-50"
  title="Save config"
>
  <Save className="h-4 w-4" />
</button>
```

- [ ] **Step 3: Move status text into a subtle top-bar region**

Replace the existing full-width bordered status block with a compact line in the top bar or immediately below it.

Constraints:

- status must remain visible
- it must not push the canvas down significantly
- error text can still be plain text

- [ ] **Step 4: Run build**

Run: `npm run build`

Expected: build passes or only fails on not-yet-implemented settings modal symbols.

- [ ] **Step 5: Commit the top bar**

```bash
git add web/src/App.tsx web/src/styles.css
git commit -m "feat: add compact admin top bar"
```

### Task 3: Create The Centered Settings Modal For Global Config, Providers, And Models

**Files:**
- Modify: `web/src/App.tsx`
- Reuse: existing provider/model editing helpers in `web/src/App.tsx`
- Verify: `web/src/App.tsx`

- [ ] **Step 1: Add modal visibility state and open/close handlers**

In `App.tsx`, add:

```tsx
const [settingsOpen, setSettingsOpen] = useState(false);
```

Use this state from the settings icon button and modal close button.

- [ ] **Step 2: Add a centered modal shell**

Implement a modal overlay directly in `App.tsx`:

```tsx
{settingsOpen ? (
  <div className="fixed inset-0 z-50 flex items-center justify-center bg-zinc-950/35 p-4">
    <div className="max-h-[85vh] w-full max-w-4xl overflow-hidden rounded-3xl border border-zinc-200 bg-white shadow-[0_30px_120px_rgba(15,23,42,0.18)]">
      ...
    </div>
  </div>
) : null}
```

- [ ] **Step 3: Add the global config section**

Inside the modal, include editable fields for:

- `listen`
- `admin_listen`
- `default_secret_env`

Use the existing `Field` helper pattern already in `App.tsx`.

- [ ] **Step 4: Move provider management into the modal**

Rework the existing provider editor surface so it renders as a section inside the modal instead of the old provider-first screen.

Minimum features:

- list providers
- add provider
- edit `id`, `name`, `base_url`
- edit default headers
- remove provider

- [ ] **Step 5: Move model management into the modal**

Rework the existing model editor surface so it renders as another modal section.

Minimum features:

- list models
- add model
- edit `id`, `name`, `provider_id`, `description`
- remove model

- [ ] **Step 6: Ensure modal edits update the same shared config state**

Keep using `setConfig` directly so changes are reflected in graph inspector dropdowns without additional sync code.

- [ ] **Step 7: Run build**

Run: `npm run build`

Expected: modal compiles and settings button opens/closes without type errors.

- [ ] **Step 8: Commit the settings modal**

```bash
git add web/src/App.tsx
git commit -m "feat: move config management into settings modal"
```

### Task 4: Expand The Canvas Surface And Simplify The Editor Framing

**Files:**
- Modify: `web/src/components/rule-graph-editor.tsx`
- Modify: `web/src/styles.css`
- Verify: `web/src/components/rule-graph-editor.tsx`

- [ ] **Step 1: Remove redundant framing around the graph editor**

Update `RuleGraphEditor` so it assumes it is the primary screen, not a nested card in a resource page.

Target changes:

- reduce outer padding
- increase canvas height
- reduce visual chrome around the editor shell
- keep the right inspector usable

- [ ] **Step 2: Increase canvas footprint**

Adjust the main canvas container from the current fixed height to a larger viewport-oriented height, for example:

```tsx
<div className="h-[calc(100vh-7.5rem)] min-h-[760px] ...">
```

Tune based on the new top bar height.

- [ ] **Step 3: Keep the compact floating toolbox**

Preserve the left icon rail and drag-to-create behavior, but make sure it visually reads as floating support UI rather than a primary column.

- [ ] **Step 4: Rebalance inspector width**

Keep the node properties panel visible, but tune the grid columns so the canvas gets more space than it currently does. For example:

```tsx
lg:grid-cols-[72px_minmax(0,1fr)_320px]
```

or similar if the modal now owns more of the heavy config editing.

- [ ] **Step 5: Run build**

Run: `npm run build`

Expected: graph editor compiles and layout classes remain valid.

- [ ] **Step 6: Commit the canvas expansion**

```bash
git add web/src/components/rule-graph-editor.tsx web/src/styles.css
git commit -m "style: prioritize canvas space in rule editor"
```

### Task 5: Replace Explicit Validate With Automatic Canvas Status

**Files:**
- Modify: `web/src/App.tsx`
- Modify: `web/src/components/rule-graph-editor.tsx`
- Verify: `web/src/components/rule-graph-editor.tsx`

- [ ] **Step 1: Remove the dedicated validate action from the toolbar**

Delete the top-bar validate button from `App.tsx`.

- [ ] **Step 2: Surface graph/global validation state as a compact canvas badge**

In `RuleGraphEditor`, keep `validateGraph(graph, config)` as the source of truth and show a top-right indicator:

- hidden or subtle when valid
- red exclamation when global issues exist

Suggested UI:

```tsx
{validation.globalIssues.length > 0 ? (
  <div className="absolute right-4 top-4 z-10 inline-flex h-9 w-9 items-center justify-center rounded-full border border-rose-200 bg-rose-50 text-rose-600 shadow-sm">
    <AlertTriangle className="h-4 w-4" />
  </div>
) : null}
```

- [ ] **Step 3: Preserve node-level validation**

Do not remove:

- unreachable node styling
- node issue counts
- inspector issue messages

- [ ] **Step 4: Keep server-side validation on save**

Do not add automatic API validation calls on every keystroke. Local visual validation is enough for the UI layer; save remains protected by backend validation.

- [ ] **Step 5: Run build**

Run: `npm run build`

Expected: no references to the old validate toolbar action remain.

- [ ] **Step 6: Commit the validation UX change**

```bash
git add web/src/App.tsx web/src/components/rule-graph-editor.tsx
git commit -m "feat: make validation automatic in canvas ui"
```

### Task 6: End-To-End Verification

**Files:**
- Verify only: `web/src/App.tsx`
- Verify only: `web/src/components/rule-graph-editor.tsx`
- Verify only: `src/admin_api.rs`

- [ ] **Step 1: Run frontend production build**

Run: `npm run build`

Expected: Vite build succeeds with no TypeScript errors.

- [ ] **Step 2: Run backend tests**

Run: `cargo test`

Expected: all Rust tests pass.

- [ ] **Step 3: Manual UI verification checklist**

Verify these behaviors locally:

- landing screen opens to the canvas directly
- top bar is thin and text-light
- right side only has icon actions for settings, load, save
- settings opens as a centered modal
- provider/model edits in the modal affect node dropdowns
- dragging nodes then saving and loading preserves positions
- validation issues show as a red indicator near the canvas top-right

- [ ] **Step 4: Commit final integration**

```bash
git add web/src/App.tsx web/src/components/rule-graph-editor.tsx web/src/styles.css src/admin_api.rs
git commit -m "feat: ship canvas-first admin experience"
```
