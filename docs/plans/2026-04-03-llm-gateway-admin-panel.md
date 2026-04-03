# LLM Gateway Admin Panel Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将当前单文件 Rust 代理重构为通用 LLM 网关，并增加基于 React + shadcn/ui 的管理面板，用于配置 providers、models、routes 和 header rules。

**Architecture:** 保持单二进制交付。Rust 负责网关转发、规则求值、配置读写和管理 API；前端单独放在 `web/` 目录构建，产物嵌入或复制到后端静态资源目录。配置落盘到 `config/gateway.toml`，请求只做路由选择、path rewrite 和 header 注入/删除/覆盖，不解析 body。

**Tech Stack:** Rust, Axum, Reqwest, Serde, TOML, React, TypeScript, Vite, shadcn/ui, Tailwind CSS

---

### Task 1: Restructure the Rust crate

**Files:**
- Create: `src/lib.rs`
- Create: `src/config.rs`
- Create: `src/crypto.rs`
- Create: `src/gateway.rs`
- Create: `src/rules.rs`
- Create: `src/admin_api.rs`
- Create: `src/frontend.rs`
- Modify: `src/main.rs`

**Step 1: Add a library entrypoint**

Create `src/lib.rs` to expose the new modules and shared app state types.

**Step 2: Move crypto code out of `main.rs`**

Move encryption, key derivation, and decryption helpers into `src/crypto.rs`.

**Step 3: Move config parsing out of `main.rs`**

Create `src/config.rs` for config structs, file loading, validation entrypoint, and atomic save helpers.

**Step 4: Move proxy path handling into `src/gateway.rs`**

Extract request routing, upstream URL building, request forwarding, and logging into the gateway module.

**Step 5: Add admin API and frontend placeholders**

Create empty-but-compiling modules for `admin_api.rs` and `frontend.rs` so `main.rs` can compose them.

**Step 6: Rebuild `src/main.rs` as composition only**

Keep `main.rs` focused on CLI parsing, config bootstrap, router assembly, and server startup.

**Step 7: Verify the crate still compiles**

Run: `cargo test`
Expected: compilation succeeds or only fails on intentionally unimplemented stubs added in this task.

**Step 8: Commit**

Run:
```bash
git add src/main.rs src/lib.rs src/config.rs src/crypto.rs src/gateway.rs src/rules.rs src/admin_api.rs src/frontend.rs
git commit -m "refactor: split gateway into modules"
```

### Task 2: Introduce the new gateway config model

**Files:**
- Modify: `src/config.rs`
- Create: `config/gateway.toml`
- Modify: `README.md`

**Step 1: Define the new config schema**

Add structs for:

- `GatewayConfig`
- `ProviderConfig`
- `ModelConfig`
- `RouteConfig`
- `HeaderRuleConfig`
- `HeaderActionConfig`
- `HeaderValueConfig`

**Step 2: Support the new file location**

Change the default CLI config path from `proxy.toml` to `config/gateway.toml`.

**Step 3: Implement semantic validation**

Add validation for:

- duplicate ids
- route references to missing provider/model
- invalid scopes
- empty match expressions when required
- invalid encrypted header definitions

**Step 4: Add atomic save helpers**

Write config save logic that serializes TOML to a temp file and renames it into place.

**Step 5: Add a default example config**

Create `config/gateway.toml` with one provider, one model, one route, and one header rule.

**Step 6: Update docs**

Replace old alias-based examples in `README.md` with the new file structure and startup flow.

**Step 7: Verify config parsing**

Run: `cargo test`
Expected: tests cover successful parsing plus invalid reference failures.

**Step 8: Commit**

Run:
```bash
git add src/config.rs config/gateway.toml README.md
git commit -m "feat: add structured gateway config"
```

### Task 3: Build the rule engine for route matching and header actions

**Files:**
- Modify: `src/rules.rs`
- Modify: `src/gateway.rs`
- Test: `src/rules.rs`

**Step 1: Define runtime request context**

Add a lightweight context struct containing method, path, query, headers, selected provider, selected model, and selected route.

**Step 2: Implement the expression evaluator**

Support only:

- `==`
- `!=`
- `&&`
- `||`
- `!`
- `.startsWith(...)`
- `.contains(...)`
- `header["name"]`

Reject any unsupported syntax with explicit errors.

**Step 3: Implement template rendering**

Add `${provider.id}`, `${model.id}`, `${route.id}`, `${request.header.Name}`, and `${env.NAME}` resolution.

**Step 4: Implement header actions**

Add action handlers for:

- `set`
- `remove`
- `copy`
- `set_if_absent`

**Step 5: Apply deterministic rule ordering**

Execute rules in `global -> provider -> model -> route` order, then in file order within each scope.

**Step 6: Add unit tests**

Write tests for:

- route expression matches
- template rendering
- remove vs set precedence
- missing template variables
- invalid expressions

**Step 7: Verify**

Run: `cargo test`
Expected: rule engine tests pass.

**Step 8: Commit**

Run:
```bash
git add src/rules.rs src/gateway.rs
git commit -m "feat: add route and header rule engine"
```

### Task 4: Upgrade the gateway runtime to use providers, models, routes, and path rewrite

**Files:**
- Modify: `src/gateway.rs`
- Modify: `src/main.rs`
- Test: `src/gateway.rs`

**Step 1: Replace alias resolution with route resolution**

Use configured routes instead of legacy alias lookup to choose provider and optional model.

**Step 2: Implement path rewrite**

Apply `path_rewrite` only after a route matches, preserving query strings.

**Step 3: Merge provider headers and rule results**

Build the final outgoing headers from:

- incoming request headers
- provider default headers
- header rules

Continue stripping hop-by-hop and internal control headers.

**Step 4: Preserve streaming responses**

Keep `bytes_stream()` based response forwarding unchanged.

**Step 5: Add runtime tests**

Cover:

- no route matched
- matching higher-priority route
- path rewrite correctness
- header removal before forwarding

**Step 6: Verify**

Run: `cargo test`
Expected: gateway tests pass and old alias assumptions are removed.

**Step 7: Commit**

Run:
```bash
git add src/gateway.rs src/main.rs
git commit -m "feat: route requests through provider and model config"
```

### Task 5: Extend encrypted header support beyond Authorization

**Files:**
- Modify: `src/crypto.rs`
- Modify: `src/config.rs`
- Modify: `src/gateway.rs`
- Modify: `README.md`
- Test: `src/crypto.rs`

**Step 1: Change encrypted header representation**

Model provider default headers as structured values, not plain strings.

**Step 2: Generalize decryption**

Allow any configured header to use `encrypted = true`.

**Step 3: Keep CLI compatibility**

Retain `encrypt-header --value --secret-env` and make the command generic rather than `Authorization`-specific in docs and validation.

**Step 4: Add tests**

Cover:

- successful decrypt for any header name
- missing env var
- malformed encrypted payload

**Step 5: Verify**

Run: `cargo test`
Expected: crypto and config tests pass.

**Step 6: Commit**

Run:
```bash
git add src/crypto.rs src/config.rs src/gateway.rs README.md
git commit -m "feat: support encrypted values for any configured header"
```

### Task 6: Add the admin API for config read, validate, save, and reload

**Files:**
- Modify: `src/admin_api.rs`
- Modify: `src/config.rs`
- Modify: `src/main.rs`
- Test: `src/admin_api.rs`

**Step 1: Define shared admin state**

Store the active config in a reloadable container that both gateway and admin API can access.

**Step 2: Implement `GET /admin/config`**

Return the current structured config as JSON.

**Step 3: Implement `POST /admin/validate`**

Validate the posted config without saving it.

**Step 4: Implement `PUT /admin/config`**

Validate and atomically persist the config to disk, then update in-memory state.

**Step 5: Implement `POST /admin/reload`**

Reload from disk and swap the in-memory config.

**Step 6: Add endpoint tests**

Cover:

- valid save
- invalid save
- reload after file change

**Step 7: Verify**

Run: `cargo test`
Expected: admin API tests pass.

**Step 8: Commit**

Run:
```bash
git add src/admin_api.rs src/config.rs src/main.rs
git commit -m "feat: add admin config api"
```

### Task 7: Scaffold the React + shadcn/ui frontend

**Files:**
- Create: `web/package.json`
- Create: `web/tsconfig.json`
- Create: `web/vite.config.ts`
- Create: `web/index.html`
- Create: `web/src/main.tsx`
- Create: `web/src/App.tsx`
- Create: `web/src/lib/types.ts`
- Create: `web/src/lib/api.ts`
- Create: `web/src/components/ui/*`
- Create: `web/src/pages/providers.tsx`
- Create: `web/src/pages/models.tsx`
- Create: `web/src/pages/routes.tsx`
- Create: `web/src/pages/header-rules.tsx`
- Create: `web/src/pages/config.tsx`
- Create: `web/src/styles.css`
- Create: `web/components.json`
- Create: `web/postcss.config.js`
- Create: `web/tailwind.config.ts`

**Step 1: Scaffold the Vite app**

Set up a TypeScript React app in `web/`.

**Step 2: Install and configure Tailwind + shadcn/ui**

Create the minimum config needed for shadcn components and consistent styling.

**Step 3: Define shared frontend types**

Mirror the admin API config model in `web/src/lib/types.ts`.

**Step 4: Add API client helpers**

Implement helpers for:

- fetch config
- validate config
- save config
- reload config

**Step 5: Build shell navigation**

Create a practical admin layout with sections for Providers, Models, Routes, Header Rules, and Config.

**Step 6: Verify frontend builds**

Run: `npm install`
Then: `npm run build`
Expected: production assets are generated.

**Step 7: Commit**

Run:
```bash
git add web
git commit -m "feat: scaffold admin panel frontend"
```

### Task 8: Implement the admin UI flows

**Files:**
- Modify: `web/src/App.tsx`
- Modify: `web/src/pages/providers.tsx`
- Modify: `web/src/pages/models.tsx`
- Modify: `web/src/pages/routes.tsx`
- Modify: `web/src/pages/header-rules.tsx`
- Modify: `web/src/pages/config.tsx`
- Modify: `web/src/lib/api.ts`
- Modify: `web/src/lib/types.ts`

**Step 1: Providers editor**

Add create/edit/delete forms for providers and their default headers.

**Step 2: Models editor**

Add provider-bound model management.

**Step 3: Routes editor**

Add fields for priority, expression, provider, model, and path rewrite.

**Step 4: Header rules editor**

Add scope selection, target selection, condition expression, and action list editing.

**Step 5: Config preview**

Add validation feedback, reload button, and raw config summary view.

**Step 6: UX polish**

Add error toasts, dirty-state warnings, and delete confirmations.

**Step 7: Verify**

Run: `npm run build`
Expected: the panel compiles without type errors.

**Step 8: Commit**

Run:
```bash
git add web/src
git commit -m "feat: implement admin panel editors"
```

### Task 9: Serve the built frontend from Rust

**Files:**
- Modify: `src/frontend.rs`
- Modify: `src/main.rs`
- Modify: `Cargo.toml`

**Step 1: Decide asset strategy**

Use embedded static assets or a copied `dist/` directory under a known runtime path. Prefer embedding for single-binary delivery.

**Step 2: Implement frontend handler**

Serve `index.html` for panel routes and static files for built assets.

**Step 3: Mount the panel**

Expose the panel under `/admin` without conflicting with `/admin/config` API routes.

**Step 4: Verify**

Run: `cargo test`
Expected: backend still compiles with frontend serving enabled.

**Step 5: Commit**

Run:
```bash
git add src/frontend.rs src/main.rs Cargo.toml
git commit -m "feat: serve admin panel from rust binary"
```

### Task 10: Finish documentation and end-to-end verification

**Files:**
- Modify: `README.md`
- Modify: `config/gateway.toml`

**Step 1: Document the new startup flow**

Include:

- encrypting header values
- starting the gateway
- opening the admin panel
- creating providers/models/routes/rules

**Step 2: Document practical examples**

Add examples for:

- model A adds a header
- model B removes a header
- route rewrites `/v1/chat/completions`

**Step 3: Run backend verification**

Run: `cargo test`
Expected: all Rust tests pass.

**Step 4: Run frontend verification**

Run: `npm run build`
Expected: frontend build succeeds.

**Step 5: Smoke test manually**

Start the app and verify:

- panel loads
- config saves
- route matches
- headers are injected/removed as configured

**Step 6: Commit**

Run:
```bash
git add README.md config/gateway.toml
git commit -m "docs: finalize gateway admin workflow"
```
