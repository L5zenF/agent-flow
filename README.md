# proxy-tools

一个本地通用 LLM 网关：

- 不适配任何协议
- 不解析 body
- 只做请求转发、path rewrite、请求头注入/删除/覆盖
- 用本地文件配置 providers / models / routes / header rules
- 自带 React + shadcn 风格管理面板

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

## 管理 API

- `GET /admin/config`
- `PUT /admin/config`
- `POST /admin/validate`
- `POST /admin/reload`

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
