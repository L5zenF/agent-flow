# proxy-tools

一个本地通用 LLM 网关：

- 不适配任何协议
- 不解析 body
- 只做请求转发、path rewrite、请求头注入/删除/覆盖
- 用本地文件配置 providers / models / routes / header rules
- 自带 React + shadcn 风格管理面板
- 支持基于 React Flow 的全局可视化规则图

## 核心能力

- `providers`
  - 定义上游基础地址和默认请求头
- `models`
  - 定义逻辑模型并绑定 provider
- `routes`
  - 按条件表达式匹配请求，选择 provider / model，并可 rewrite path
- `header_rules`
  - 按作用域和条件表达式执行头部动作
- 密文字段
  - 任意 provider 默认头部都可以加密存储
- `wasm_plugin`
  - 用 WASM 组件扩展单个图节点能力，流程编排仍由用户拖拽节点完成

## 配置文件

默认配置文件路径：

```text
config/gateway.toml
```

示例：

```toml
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
default_secret_env = "PROXY_SECRET"

[[providers]]
id = "kimi"
name = "Kimi"
base_url = "https://api.kimi.com"

[[providers.default_headers]]
name = "Authorization"
value = "Bearer replace-me"

[[models]]
id = "kimi-k2"
name = "Kimi K2"
provider_id = "kimi"

[[routes]]
id = "chat-default"
priority = 100
enabled = true
match = 'path.startsWith("/v1/chat/completions") && method == "POST"'
provider_id = "kimi"
model_id = "kimi-k2"
path_rewrite = "/coding/v1/chat/completions"

[[header_rules]]
id = "inject-model-header"
enabled = true
scope = "model"
target_id = "kimi-k2"
when = 'path.startsWith("/v1/")'

[[header_rules.actions]]
type = "set"
name = "X-Model"
value = "${model.id}"
```

## 规则表达能力

支持的条件表达式：

- `method == "POST"`
- `method != "GET"`
- `path.startsWith("/v1/")`
- `path.contains("/chat/")`
- `header["x-target"] == "kimi"`
- `&&` / `||` / `!`

支持的头部动作：

- `set`
- `remove`
- `copy`
- `set_if_absent`

支持的模板变量：

- `${provider.id}`
- `${provider.name}`
- `${model.id}`
- `${route.id}`
- `${request.header.Authorization}`
- `${env.PROXY_SECRET}`

## 可视化规则图

管理面板新增 `Rule Graph` 视图：

- 一张全局图，所有请求先进图
- 图中节点负责：
  - 条件分支
  - 选择 provider
  - 选择 model
  - path rewrite
  - 设置 / 删除 / 复制请求头

当前节点类型：

- `start`
- `condition`
- `wasm_plugin`
- `route_provider`
- `select_model`
- `rewrite_path`
- `set_header`
- `remove_header`
- `copy_header`
- `set_header_if_absent`
- `end`

当前条件节点支持：

- `expression`
- `builder` 数据模型已预留，MVP 阶段以前端表达式编辑为主

如果 `config/gateway.toml` 中存在 `rule_graph`，运行时优先走图；没有图时才回退到旧的 `routes` / `header_rules`。

## 启动

启动网关和管理面板：

```bash
cargo run -- --config config/gateway.toml
```

默认监听：

- gateway: `127.0.0.1:9001`
- admin: `127.0.0.1:9002`

打开管理面板：

```text
http://127.0.0.1:9002/admin/ui
```

Workflow 入口：

- 打开面板后先进入 `Workflow Gallery`
- 选中一个 workflow 后进入画布编辑
- `Set Active` 会切换当前运行中的 workflow
- `Save` 会同时保存当前 workflow 文件和全局 `gateway.toml`

## 管理 API

- `GET /admin/config`
- `GET /admin/workflows`
- `GET /admin/workflows/:id`
- `GET /admin/plugins`
- `POST /admin/workflows`
- `POST /admin/workflows/:id/activate`
- `PUT /admin/workflows/:id`
- `PUT /admin/config`
- `POST /admin/validate`
- `POST /admin/reload`

## Workflow 目录

当前结构已经拆成“主配置 + workflow 文件”：

```text
config/
  gateway.toml
workflows/
  default.toml
  chat-routing.toml
plugins/
  ...
```

其中：

- `config/gateway.toml` 保存全局监听地址、providers、models、workflow 索引和 `active_workflow_id`
- `workflows/*.toml` 每个文件只保存一张图
- 运行时只执行当前 `active_workflow_id` 对应的 workflow

最小主配置示例：

```toml
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
workflows_dir = "workflows"
active_workflow_id = "default"

[[workflows]]
id = "default"
name = "Default Workflow"
file = "default.toml"
description = "Starter workflow"
```

## WASM 插件

本地插件目录结构：

```text
plugins/
  intent-classifier/
    plugin.toml
    Cargo.toml
    src/lib.rs
    wit/world.wit
    wasm/plugin.wasm
```

当前仓库自带一个 sample plugin：`plugins/intent-classifier`。

构建命令：

```bash
cd plugins/intent-classifier
CARGO_COMPONENT_CACHE_DIR=../../.cargo-component-cache cargo component build --release
mkdir -p wasm
cp target/wasm32-wasip1/release/intent_classifier.wasm wasm/plugin.wasm
```

示例规则图节点：

```toml
[[rule_graph.nodes]]
id = "intent-plugin"
type = "wasm_plugin"

[rule_graph.nodes.position]
x = 320.0
y = 180.0

[rule_graph.nodes.wasm_plugin]
plugin_id = "intent-classifier"
timeout_ms = 20
max_memory_bytes = 16777216
granted_capabilities = ["log"]

[[rule_graph.edges]]
id = "intent-plugin-chat"
source = "intent-plugin"
source_handle = "chat"
target = "select_model-8"

[[rule_graph.edges]]
id = "intent-plugin-default"
source = "intent-plugin"
source_handle = "default"
target = "end-7"
```

这个 sample plugin 会：

- 读取当前 `path`
- 检查请求头 `x-intent`
- 设置 `ctx.intent`
- 返回 `chat` 或 `default` 分支

另一个更实用的 sample plugin 是 `plugins/remote-policy-router`。

它会：

- 读取本地 JSON 路由策略文件
- 可选请求远端 HTTP JSON 做覆盖
- 根据 `x-tenant` 之类的 header 或 path prefix 选择 `chat`、`embedding`、`moderation`、`default`
- 设置 `ctx.route_key` 和 `ctx.policy_source`

构建命令：

```bash
cd plugins/remote-policy-router
CARGO_COMPONENT_CACHE_DIR=../../.cargo-component-cache cargo component build --release
mkdir -p wasm
cp target/wasm32-wasip1/release/remote_policy_router.wasm wasm/plugin.wasm
```

示例节点配置：

```toml
[rule_graph.nodes.wasm_plugin]
plugin_id = "remote-policy-router"
timeout_ms = 50
fuel = 1000000
max_memory_bytes = 16777216
granted_capabilities = ["fs", "network", "log"]
read_dirs = ["plugins/remote-policy-router/examples"]
allowed_hosts = ["127.0.0.1:9100"]

[rule_graph.nodes.wasm_plugin.config]
policy_file = "/plugins/remote-policy-router/examples/local-policy.json"
policy_url = "http://127.0.0.1:9100/policy"
match_header = "x-tenant"
fallback_port = "default"
```

## 请求转发示例

按配置命中 route 后，客户端原样请求网关：

```bash
curl -N http://127.0.0.1:9001/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{"stream":true}'
```

网关会：

- 根据 `routes` 选择上游 provider / model
- 按需将 `/v1/chat/completions` rewrite 到 `/coding/v1/chat/completions`
- 注入 provider 默认头
- 执行 model / route / global 级别的 header rules
- 原样透传 SSE 或 chunked 响应

## 模型头部策略示例

模型 A 添加头：

```toml
[[header_rules]]
id = "model-a-add-header"
enabled = true
scope = "model"
target_id = "model-a"

[[header_rules.actions]]
type = "set"
name = "X-Upstream-Flag"
value = "enabled"
```

模型 B 删除头：

```toml
[[header_rules]]
id = "model-b-remove-header"
enabled = true
scope = "model"
target_id = "model-b"

[[header_rules.actions]]
type = "remove"
name = "X-Upstream-Flag"
```

## 加密头部

先准备密钥环境变量：

```bash
export PROXY_SECRET='change-me'
```

生成密文：

```bash
cargo run -- encrypt-header \
  --secret-env PROXY_SECRET \
  --value 'Bearer real-upstream-token'
```

然后把配置改成：

```toml
[[providers.default_headers]]
name = "Authorization"
value = "enc:v1:..."
encrypted = true
secret_env = "PROXY_SECRET"
```

## 验证

后端测试：

```bash
cargo test
```

前端构建：

```bash
cd web
npm install
npm run build
```
