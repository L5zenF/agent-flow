# 通用 LLM 网关设计

**日期**: 2026-04-03

## 目标

把当前基于 alias 的本地代理重构为一个通用 LLM 网关：

- 不适配任何上游协议
- 不解析或改写请求体
- 只负责请求转发、路由选择、路径重写、请求头注入/删除/覆盖
- 提供一个实用导向的可视化面板，用于管理供应商、模型、路由和请求头策略
- 配置保存在本地文件中，由面板读写

## 非目标

- 不做 OpenAI / Anthropic / Gemini 等协议适配
- 不做请求体字段级别改写
- 不做响应体改写
- 不做计费、鉴权、租户、审计日志系统
- 不做复杂脚本执行引擎

## 当前现状

当前项目是一个单文件 Rust 程序：

- 从 `proxy.toml` 读取配置
- 通过 alias 选择上游
- 注入额外请求头
- 支持对 `Authorization` 做密文存储与启动时解密
- 原样流式转发响应

当前问题：

- 配置模型过于扁平，只适合少量 alias
- 路由能力弱，无法表达 provider / model / rule 的层次
- 规则能力基本只有静态 headers
- 没有管理面板
- `src/main.rs` 职责混杂，不适合继续叠功能

## 方案选择

采用“轻量重构”的单二进制方案：

- Rust 后端拆模块
- 同一进程同时承载网关转发能力和管理 API
- React + shadcn 面板构建为静态资源，由 Rust 提供
- 配置保存为本地单文件 `config/gateway.toml`

不采用前后端完全分离项目。当前项目体量小，单二进制更实用，部署成本更低。

## 高层架构

### 后端模块

- `config`
  - 负责配置文件结构、解析、校验、原子写入、热重载
- `gateway`
  - 负责接收请求、匹配路由、构建上游请求、转发响应
- `rules`
  - 负责规则条件求值、模板变量渲染、头部动作执行
- `crypto`
  - 负责加解密密文字段
- `admin_api`
  - 负责面板配置读写、校验、重载接口
- `frontend`
  - 负责嵌入和提供前端静态资源

### 运行模型

- 用户请求进入网关入口
- 路由系统根据条件选出 provider、model 和可选 path rewrite
- 规则系统根据 provider/model/route/request 上下文执行头部动作
- 网关将修改后的请求转发到上游
- 响应原样返回，保持 SSE / chunked 流式输出

## 配置模型

配置文件路径统一为 `config/gateway.toml`。

### 顶层结构

```toml
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"

[[providers]]
id = "kimi"
name = "Kimi"
base_url = "https://api.kimi.com"

  [providers.default_headers]
  Authorization = { encrypted = true, value = "enc:v1:..." }
  User-Agent = { value = "Gateway/1.0" }

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

[[header_rules.actions]]
type = "remove"
name = "X-Debug"
```

### providers

字段：

- `id`: 稳定标识
- `name`: 展示名称
- `base_url`: 上游基础地址
- `default_headers`: provider 默认头部

说明：

- provider 表示一个具体上游服务商或网关地址
- provider 默认头部会在命中该 provider 时参与最终头部计算

### models

字段：

- `id`
- `name`
- `provider_id`
- `description` 可选

说明：

- model 是逻辑模型定义，不要求和任何协议字段绑定
- model 本身不解析 body，只作为规则目标和路由结果

### routes

字段：

- `id`
- `priority`
- `enabled`
- `match`
- `provider_id`
- `model_id` 可选
- `path_rewrite` 可选

说明：

- routes 按 `priority` 从高到低匹配
- 第一个命中的 route 生效
- route 只做目标选择和 path rewrite，不处理 body

### header_rules

字段：

- `id`
- `enabled`
- `scope`: `global | provider | model | route`
- `target_id`：当 scope 非 global 时必填
- `when`：可选条件表达式
- `actions`

动作：

- `set`
- `remove`
- `copy`
- `set_if_absent`

说明：

- 全局规则先执行，之后按 provider、model、route 顺序执行
- 后执行的规则可以覆盖前面的结果

## 表达式与模板

规则系统采用“声明式动作 + 少量表达式”。

### 条件表达式

支持的上下文：

- `method`
- `path`
- `query`
- `header`
- `provider`
- `model`
- `route`

支持的表达方式限定为简单布尔表达式：

- 相等比较：`method == "POST"`
- 字符串前缀：`path.startsWith("/v1/")`
- 字符串包含：`header["x-target"].contains("kimi")`
- 逻辑运算：`&&`、`||`、`!`

不支持：

- 自定义函数
- 任意脚本
- 访问请求体
- 网络调用

### 值模板

头部值支持模板变量：

- `${provider.id}`
- `${provider.name}`
- `${model.id}`
- `${route.id}`
- `${request.header.Authorization}`
- `${env.PROXY_SECRET}`

模板渲染失败时，默认使该条规则报错并拒绝请求，避免静默注入错误头部。

## 密文头部策略

当前实现只支持加密 `Authorization`，新设计将扩展为任意头部都可标记为：

```toml
Authorization = { encrypted = true, value = "enc:v1:..." }
```

约束：

- 只允许在配置文件中声明密文字段
- 面板不显示密文原文
- 面板可以更新密文字段，但不会回显解密后的明文
- 运行时按配置标记解密后注入头部

CLI 保留加密能力，扩展为通用命令：

- `encrypt-header --value ... --secret-env ...`

输出仍是 `enc:v1:...`

## 请求处理流程

1. 接收请求
2. 读取方法、路径、查询、头部
3. 按优先级匹配 route
4. 根据 route 选择 provider 和可选 model
5. 计算上游 URL，必要时执行 path rewrite
6. 合并和执行头部策略
   - 过滤 hop-by-hop headers
   - 过滤内部控制头，如 `X-Target`
   - 应用 provider 默认头
   - 应用 header rules
7. 发送请求到上游
8. 原样回传响应头和流式响应体

## 管理 API

面板通过管理 API 操作配置。

接口：

- `GET /admin/config`
  - 返回完整配置
- `PUT /admin/config`
  - 写入并校验配置
- `POST /admin/validate`
  - 仅校验，不落盘
- `POST /admin/reload`
  - 重新从磁盘加载配置

写入策略：

- 先解析
- 再做语义校验
- 通过后写入临时文件
- 原子替换正式配置

## 前端面板

技术：

- React
- shadcn/ui
- 单独前端目录构建，产物嵌入 Rust

页面范围：

- Providers
  - 列表
  - 新增/编辑/删除
  - 默认头部配置
- Models
  - 列表
  - 新增/编辑/删除
  - 绑定 provider
- Routes
  - 列表
  - 优先级排序
  - 条件表达式
  - provider/model 目标选择
  - path rewrite
- Header Rules
  - 列表
  - 作用域选择
  - 条件表达式
  - 动作列表编辑
- Config
  - 原始 TOML 预览
  - 导入导出
  - 校验结果展示

UI 原则：

- 表单优先，不把面板做成脚本 IDE
- 高级能力集中在“条件表达式”文本框
- 常见动作全部结构化录入
- 所有删除操作有确认

## 路由与规则优先级

### route 优先级

- 数值越大优先级越高
- 相同优先级按文件顺序

### header rule 执行顺序

执行顺序固定：

1. global
2. provider
3. model
4. route

每个层级内：

- 先按显式顺序
- 再按 action 顺序执行

这样可以保证覆盖行为可预测。

## 错误处理

以下情况返回 4xx：

- 没有命中 route
- route 引用了不存在的 provider/model
- 条件表达式非法
- 模板变量渲染失败
- 需要解密但缺少环境变量

以下情况返回 5xx / 502：

- 上游连接失败
- 配置加载失败
- 响应构建失败

错误信息应尽量指向具体配置项，例如：

- `route 'chat-default' references missing provider 'kimi'`
- `header_rule 'inject-model-header' template variable 'model.id' is unavailable`

## 可观测性

日志至少包含：

- route id
- provider id
- model id
- method
- path
- upstream url
- 修改过的头部名称列表

约束：

- 不打印敏感头部值
- 密文和明文 token 都不进入日志

## 向后兼容策略

不强求兼容旧格式。

理由：

- 当前配置格式太扁平，不适合作为长期形态
- 继续兼容会显著增加实现复杂度

迁移方式：

- 提供新的默认配置示例
- README 更新为新结构

## 实施顺序

1. 重构 Rust 代码结构并引入新配置模型
2. 实现 route 匹配、path rewrite、header rule 引擎
3. 扩展密文字段能力
4. 增加管理 API
5. 搭建 React + shadcn 面板
6. 连接面板与管理 API
7. 更新 README 和示例配置

## 风险与取舍

### 1. 表达式能力失控

风险：

- 一旦支持完整脚本，网关会变成不可控运行时

取舍：

- 仅支持受限布尔表达式和模板变量

### 2. 前端引入复杂度

风险：

- 当前是纯 Rust 项目，引入前端构建链会增加维护成本

取舍：

- 接受这个成本，因为管理面板是明确目标
- 但仍保持单二进制交付

### 3. 配置文件并发写入

风险：

- 面板保存过程中可能写坏配置

取舍：

- 使用校验 + 临时文件 + 原子替换

## 验收标准

- 可以通过面板添加 provider
- 可以通过面板添加 model 并绑定 provider
- 可以通过面板添加 route，按条件匹配请求并转发到指定 provider/model
- 可以通过面板添加 header rule，实现加头、删头、拷贝头、按条件注入
- 支持 path rewrite
- 不解析 body，不做协议适配
- 支持流式响应透传
- 配置持久化到本地文件
- 启动后能成功加载新配置并处理请求
