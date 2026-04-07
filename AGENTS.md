# Repository Guidelines

## Project Structure & Module Organization
`src/` contains the Rust gateway and admin API. Key modules include `gateway.rs` for request proxying, `config.rs` for TOML-backed types, `rules.rs` for matching logic, and `frontend.rs` for serving the bundled admin UI. Runtime configuration lives in `config/gateway.toml`. The React/Vite admin app is isolated under `web/`; reusable helpers live in `web/src/lib`, and UI components live in `web/src/components`.

## Build, Test, and Development Commands
Use `cargo run -- --config config/gateway.toml` to start the gateway and admin UI locally. Use `cargo test` for the Rust test suite and `cargo fmt` before submitting backend changes. For the frontend, run `cd web && npm install` once, `npm run dev` for local UI work, and `npm run build` to produce a production bundle and type-check with `tsc -b`.

## Coding Style & Naming Conventions
Follow Rust defaults: 4-space indentation, `snake_case` for functions/modules, `PascalCase` for types, and keep modules focused by domain. Always format backend code with `cargo fmt`. In `web/`, use TypeScript with React function components, `PascalCase` component names, and `camelCase` for hooks, helpers, and state. Existing frontend code uses Tailwind utilities and `@/` path aliases; keep new code consistent with that pattern.

## Testing Guidelines
There is no large standalone test tree yet, so add Rust unit tests near the module they cover or add integration tests under `tests/` when behavior spans modules. For frontend changes, at minimum run `npm run build` to catch TypeScript and bundling regressions. Validate user-facing gateway changes against `config/gateway.toml` and, when relevant, exercise `/admin/config` or `/admin/validate` locally.

## Commit & Pull Request Guidelines
Recent history mixes plain imperative subjects with Conventional Commit prefixes such as `feat:`, `fix:`, `docs:`, and `style:`. Prefer short imperative messages and use a prefix when it clarifies intent, for example `feat: add route-provider node validation`. Pull requests should describe the behavior change, list verification steps run, link related issues, and include screenshots for admin UI updates.

## Security & Configuration Tips
Do not commit real upstream secrets. Keep encrypted header values in config and load keys from environment variables such as `PROXY_SECRET`. If you touch provider headers, routing, or config persistence, verify both gateway traffic and admin save/reload flows before merging.
