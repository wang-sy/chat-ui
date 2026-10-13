# Chat UI × Codex app-server 能力清单

这份清单按本机 `codex-cli 0.153.4` 在 2026-10-12 生成的 app-server v2 schema 校对，代码只使用已经在本地运行时验证过的字段。Chat UI 通过一个 Rust 进程管理一个 stdio JSON-RPC 会话，浏览器通过本地服务读取同一份状态。

## 已接入

| Codex 能力 | Chat UI 接入 | 用户可见结果 |
| --- | --- | --- |
| `initialize` / `initialized` | Rust 启动 app-server 时协商客户端身份 | 服务只建立一个共享 Codex 会话 |
| `model/list` | 连接后分页读取模型目录 | 模型、effort、速度统一选择器 |
| `thread/start` / `thread/resume` / `thread/read` | Main Thread 与每个 View Thread 分开持久化和恢复 | Space 结构与对话历史在重启后保持 |
| `turn/start` / `turn/interrupt` | Main、View 都走真实 turn 生命周期 | 开始、停止、完成、失败、重连均可恢复 |
| `turn/steer` | 运行中的线程优先尝试追加消息；协议拒绝时回退到本地队列 | 运行中继续发送，不丢消息 |
| `item/agentMessage/delta` | Rust 聚合增量文本并通过 SSE 推送快照 | 助手回复逐字流式出现 |
| `item/started` / `item/completed` | 识别回答、命令、文件变更等 item | 状态文案随实际工作阶段变化 |
| `thread/status/changed` | 识别 active、等待审批、等待输入、idle、system error | 对话状态不会只依赖轮询猜测 |
| `turn/plan/updated` / plan、reasoning 增量 | 保存当前计划和推理阶段 | 对话面板显示计划步骤，推理只显示阶段状态，不暴露隐式思维 |
| `thread/tokenUsage/updated` | 保留当前 turn 的 token 使用快照 | 对话面板显示本回合 token 用量 |
| `item/commandExecution/outputDelta` | 识别命令执行输出阶段 | 运行命令时显示准确进度 |
| `item/mcpToolCall/progress` | 使用 MCP 自报进度文案 | MCP 工具调用不会显示成无意义的“思考中” |
| `model/rerouted` | 更新当前模型并提示切换原因 | 模型被服务端重路由时状态透明 |
| approval / user input / MCP elicitation requests | 保留原始 request id，统一在 UI 回复 | 命令、文件变更、权限、表单和 MCP 请求可继续执行 |
| 事件流 | `GET /api/events` 使用 SSE，带序号、断线续接和心跳 | 不再等待 900ms 状态轮询才看到变化 |
| 本地排队、引导、编辑、删除、拖动排序 | 浏览器本地持久化，发送前捕获模型/effort/速度/权限 | 多线程排队可恢复，失败项可修正 |
| Main/View 结构化结果 | Main 只负责 Space 计划，View 只负责页面 | 页面编排和页面实现有清晰职责边界 |

## 选择性保留的能力

Codex 还提供原生 `thread/queue/*`。Chat UI 目前使用自己的队列层：它需要同时保存 View 目标、模型、effort、速度和权限，并支持跨线程排序；原生队列只保存 thread 级输入，无法表达这些产品字段。真正发送到活动线程时仍优先使用 `turn/steer`，因此不会绕过 Codex 的 turn 语义。

`turn/settings/update` 只影响正在运行的 turn，且不会回写后续 turn 的默认设置。当前选择器遵循 Codex 的安全预期，把模型、effort 和速度作为下一条消息的快照；运行中的操作不会因为用户误触而改变。若产品后续需要“实时调参”，可以在确认式交互中单独接入这个方法。

Codex 的实时语音 (`thread/realtime/*`)、音频输出、独立进程/文件系统 API、远程控制、登录和账户接口也存在，但它们会引入独立权限、凭证或音频生命周期。当前产品明确移除了不好用的语音入口；agent 仍通过 app-server 自己的沙箱、审批和 MCP 能力访问文件与工具，Chat UI 不绕过这些安全边界去直接执行命令。

## 事件与恢复约定

事件消息包含 `sequence`、原始 `method`/`params` 和发送时的完整状态快照。Codex 原生通知、审批响应、View 编排、Space 变更、断线和错误都走同一个事件日志。浏览器用 `Last-Event-ID` 断线续接，Rust 端保留最近 512 个事件；SSE 不可用时保留定时状态轮询作为兼容回退。状态快照以 Codex 通知为准，Space 页面仍通过版本号和原子持久化收敛。

## 依据

- [Codex app-server README](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)
- [OpenAI Codex app-server / ChatGPT plan](https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server)
- [Codex Python SDK API reference](https://github.com/openai/codex/blob/main/sdk/python/docs/api-reference.md)
- [Codex app-server v2 turn protocol](https://github.com/openai/codex/blob/main/codex-rs/app-server-protocol/src/protocol/v2/turn.rs)
