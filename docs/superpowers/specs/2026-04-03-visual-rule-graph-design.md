# 可视化规则图设计

**日期**: 2026-04-03

## 目标

在当前通用 LLM 网关上增加一个全局可视化规则图：

- 前端使用 `React Flow`
- 所有请求进入同一张全局规则图
- 规则图负责：
  - 选择 provider
  - 选择 model
  - path rewrite
  - 请求头 set / remove / copy / set_if_absent
- 条件节点支持“可视化配置 + 少量表达式”

## 非目标

- 不做任意工作流编排
- 不做脚本节点
- 不解析或修改 body
- 不做协议适配
- 不做 Coze 式通用自动化平台

## 为什么要做

当前规则模型虽然已经支持路由和 header rules，但表达方式仍然偏配置表单：

- 分支关系不直观
- 执行顺序不直观
- 规则一多后，维护成本高
- 用户很难快速看懂“请求是怎么走的”

而网关的核心问题本质上是：

- 请求如何分流
- 请求如何在分流过程中被改头

这类逻辑天然更适合图，而不是线性表单。

## 总体方案

采用“一张全局规则图 + 受限节点执行器”方案：

- 前端用 `React Flow` 编辑图
- 后端不直接执行前端图对象
- 后端将图编译为受限执行计划
- 运行时按节点顺序和条件分支执行

## 配置模型

在现有 `config/gateway.toml` 中增加新的顶层结构，例如：

```toml
[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 120, y = 120 }

[[rule_graph.nodes]]
id = "cond-1"
type = "condition"
position = { x = 360, y = 120 }

  [rule_graph.nodes.condition]
  mode = "expression"
  expression = 'path.startsWith("/v1/chat/completions") && method == "POST"'

[[rule_graph.nodes]]
id = "provider-kimi"
type = "route_provider"
position = { x = 640, y = 40 }

  [rule_graph.nodes.route_provider]
  provider_id = "kimi"

[[rule_graph.nodes]]
id = "model-k2"
type = "select_model"
position = { x = 920, y = 40 }

  [rule_graph.nodes.select_model]
  model_id = "kimi-k2"

[[rule_graph.nodes]]
id = "rewrite-chat"
type = "rewrite_path"
position = { x = 1200, y = 40 }

  [rule_graph.nodes.rewrite_path]
  value = "/coding/v1/chat/completions"

[[rule_graph.nodes]]
id = "set-header"
type = "set_header"
position = { x = 1480, y = 40 }

  [rule_graph.nodes.set_header]
  name = "X-Model"
  value = "${model.id}"

[[rule_graph.nodes]]
id = "end"
type = "end"
position = { x = 1760, y = 40 }

[[rule_graph.edges]]
id = "e1"
source = "start"
target = "cond-1"

[[rule_graph.edges]]
id = "e2"
source = "cond-1"
source_handle = "true"
target = "provider-kimi"

[[rule_graph.edges]]
id = "e3"
source = "provider-kimi"
target = "model-k2"
```

## 节点类型

### 1. Start

- 图唯一入口
- 每张图必须且只能有一个

### 2. Condition

职责：

- 根据请求上下文做真假分支

模式：

- `builder`
  - 表单选择字段、操作符和值
- `expression`
  - 手写受限表达式

输出：

- `true`
- `false`

### 3. Route Provider

职责：

- 设置当前请求的目标 provider

字段：

- `provider_id`

### 4. Select Model

职责：

- 设置当前请求的目标 model

字段：

- `model_id`

### 5. Rewrite Path

职责：

- 改写请求 path

字段：

- `value`

### 6. Set Header

职责：

- 设置或覆盖头部

字段：

- `name`
- `value`

### 7. Remove Header

职责：

- 删除头部

字段：

- `name`

### 8. Copy Header

职责：

- 从一个 header 拷贝到另一个

字段：

- `from`
- `to`

### 9. Set Header If Absent

职责：

- 仅在头部不存在时设置

字段：

- `name`
- `value`

### 10. End

- 图结束
- 输出最终执行结果

## 条件表达能力

支持上下文：

- `method`
- `path`
- `header`
- `provider`
- `model`

支持表达式：

- `==`
- `!=`
- `&&`
- `||`
- `!`
- `.startsWith(...)`
- `.contains(...)`
- `header["name"]`

不支持：

- 自定义函数
- 任意脚本
- 访问 body
- 网络调用

## 模板变量

动作节点中的值支持：

- `${provider.id}`
- `${provider.name}`
- `${model.id}`
- `${request.header.Authorization}`
- `${env.PROXY_SECRET}`

## 执行模型

运行时上下文包含：

- 当前 provider
- 当前 model
- 当前 path
- 当前 headers

执行规则：

1. 从 `start_node_id` 开始
2. 沿边遍历
3. 条件节点根据真假走不同边
4. 动作节点顺序修改上下文
5. 到达 `end` 或无后继节点时结束

约束：

- 禁止死循环
- 禁止无效引用
- Condition 节点最多两条逻辑边
- Start 节点必须可达
- End 节点建议至少一个

## 校验规则

后端保存前必须校验：

- 是否存在唯一 start
- 所有 edge 的 source/target 是否存在
- provider/model 引用是否存在
- condition 输出边是否合法
- 图是否有环
- 节点类型参数是否完整

## 前端交互

前端新增一个 `Rule Graph` 主视图：

- 左侧节点面板
- 中间 React Flow 画布
- 右侧节点属性面板

用户操作：

- 拖拽节点到画布
- 连线
- 选择节点后编辑参数
- 保存时序列化为图配置

交互原则：

- 主视图专注图编辑
- 只暴露有限节点类型
- 属性面板不做复杂脚本编辑器

## 与现有规则系统的关系

现有 `routes` 和 `header_rules` 将不再作为长期主入口。

建议迁移策略：

- 新增 `rule_graph`，作为新主规则系统
- 旧 `routes` / `header_rules` 暂时保留只读兼容或导入入口
- 面板主入口改成图编辑器

最终方向：

- 图是主编辑方式
- 表单规则退为兼容或辅助

## 管理 API

新增或扩展接口：

- `GET /admin/config`
  - 返回 `rule_graph`
- `PUT /admin/config`
  - 保存图配置
- `POST /admin/validate`
  - 校验图结构和业务引用

可选新增：

- `POST /admin/rule-graph/compile`
  - 返回编译结果和执行计划摘要

## 风险与取舍

### 1. 图过度泛化

风险：

- 一旦支持脚本节点，系统会失控

取舍：

- 只做受限节点系统

### 2. 可视化体验很好，但执行器复杂

风险：

- 只是前端画得漂亮，没有稳定执行模型

取舍：

- 后端先定义编译和执行约束，再接前端

### 3. 与旧规则共存时间过长

风险：

- 面板出现两套规则入口

取舍：

- 明确图为主入口，旧规则只做迁移过渡

## 验收标准

- 面板可以可视化创建规则图
- 可以拖拽节点并连线
- 条件节点支持表单模式和表达式模式
- 图可以选择 provider / model / path rewrite / header actions
- 保存后后端能完成校验
- 运行时请求可以按图正确分支执行
- 不做 body 改写，不做协议适配
