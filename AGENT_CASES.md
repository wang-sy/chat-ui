# Main Thread / View Thread 验收场景

这 10 个场景是 Chat UI 的 Agent 回归基线。每个场景都检查主线程的全局编排、View Thread 的局部交付和 Space 的可恢复状态。

图形表达的选型另有 D01–D10：只给目标和事实，不指定页面或 SVG，包含表单与简短回答两个对照。真实执行与浏览器结果见 [VISUAL_AGENT_ACCEPTANCE.md](VISUAL_AGENT_ACCEPTANCE.md)，不与本组生命周期检查混为一谈。

2026-10-12 已在十个独立 Space 中进行真实 Codex 执行与浏览器验收，具体效果、修复和未通过项见 [真实执行验收报告](LIVE_AGENT_CASE_RESULTS.md)。下面的自动化矩阵属于代码回归覆盖，不能代替该报告中的真实体验验证。

| 编号 | 用户目标 | Main Thread 必须做什么 | View Thread 必须做什么 | 验收信号 |
| --- | --- | --- | --- | --- |
| C01 | “解释召回和排序的区别” | 直接回答，返回 `plan: null` | 不创建 | 不因为普通问答产生 View |
| C02 | “做一个推荐系统工作台，包含架构图和指标页” | 分派两个稳定 ID 的 View，直接内嵌成果、说明有方向的业务关系，交付后完成一次整合 | 架构以交互 SVG 图呈现，指标以图表呈现，使用共同 entityId 联动 | Main 同时展示真实子图；节点选择同步指标；单独更新不重载其他 View |
| C03 | “只给指标页增加日期筛选” | 复用指标 View ID，只调度该 View | 保留原页面能力，增加筛选和空状态 | 其他 View 的 HTML、版本、布局不变 |
| C04 | 用户点击架构图中的“排序服务” | 接收 View 事件并路由到正确 Thread | 用受限 `postMessage` 报告动作 | 点击有即时反馈，不需要复制上下文 |
| C05 | “删除旧方案，并把结果页移到右侧” | 仅按 `removeViews` 删除，重建完整连接快照，取消被删 View 的运行 | 不修改其他页面结构 | 被删 View、连接和残留页面目录均消失 |
| C06 | 同时生成三个页面，其中一个较慢 | 分别跟踪子任务，不阻塞已完成页面 | 各自发布 `working`、`ready` 或 `error` | 快页面先可用，Main 不提前宣布全部完成 |
| C07 | 生成中追加要求、重复点击发送、停止 | 追加要求进入有序队列，直接发送按 request ID 去重 | 停止后不发布过期结果，队列保留并在空闲后继续 | 不重复执行、不覆盖新版本，队列顺序可调整 |
| C08 | 子线程超时、返回空 HTML 或无效 JSON | 保留旧页面，标记失败，允许只重试失败 View | 只有完整页面才可报告 `ready` | 不出现“就绪但空白”的 View |
| C09 | 子线程需要批准或补充信息 | 在所属 Thread 展示请求，回答后继续原 turn | 不绕过权限、不把请求转给错误 Thread | 审批和输入都能完成，状态正确恢复 |
| C10 | 工作中刷新、断线重连、重启服务 | 根据 Space 注册表恢复 Main/View 关系和最新状态 | 继续原 Thread，不重复创建 | 页面、布局、版本、Thread ID 可恢复 |

## 统一边界

- Main Thread 只拥有 Space 计划、Main Page、View 注册表、版本和连接元数据。
- View Thread 只拥有自己的 HTML、局部状态和局部事件，不直接写其他 View 或主页面结构。
- `plan: null` 是明确的无空间变更；缺失或格式错误的计划必须显示可重试错误。
- View 只有在返回完整 HTML 和合法状态后才能进入 `ready`；失败时保留上一个可用页面。
- 计划中的 `connections` 是完整快照；删除必须通过显式 `removeViews`，省略 View 不代表删除。
- 页面事件只能通过受限 `postMessage` 进入宿主，并限制事件名和 payload 大小。
- 执行态不是聊天历史：Codex 的流式回复、思考摘要、计划、命令/文件输出和工具进度写入当前 Thread 的短状态，并在页面底部以 running rail 展示；回合结束后自动收起，排队、审批和错误仍保留在对话框中。
- Main Thread 在子 View 处于 `planning` 或 `working` 时保持“等待 View”状态，所有 View 完成、失败或被停止后才回到就绪，避免主页面仍在构建而 UI 提前显示空闲。

## 可执行验证矩阵

这些 case 不是只靠源码正则检查。每次回归会同时运行 Rust 状态机测试和前端行为测试：

| Case | 可执行验证 | 关键断言 |
| --- | --- | --- |
| C01 | `protocol::case_c01_main_plan_parser...` | `plan: null` 是无变更；缺失或空计划是错误 |
| C02 | `space::case_c02_main_plan...`、`space::composition`、`tests/composition.test.mjs` | 稳定引用与关系、真实完成后一次整合、禁止重新分派、来源/归属和载荷边界 |
| C03 | `space::case_c03_targeted_plan...` | 只复用目标 View，其他 View 的 HTML、版本和布局不变 |
| C04 | `tests/agent-cases.behavior.test.mjs` | `focus`、`request-main`、`action` 通过受限路由进入正确 Thread |
| C05 | `space::case_c05_main_plan_removes...` | 显式删除会清理 View、连接和选中态，其他 View 保留 |
| C06 | `events_tests::case_c06_*` | 流式思考、命令、回复细节可见；Main 等待 planning/working View |
| C07 | 前端队列与 steer 行为测试 | 20 条上限、引导/编辑/删除/拖动、活动回合 steer 和 request ID |
| C08 | `space::case_c08_view_page_update...` | 空 HTML 不能 ready；失败保留上一版可运行页面 |
| C09 | `events_tests::case_c09_*` + `requests::case_c09_*` | 审批/输入等待态和 accept/decline/cancel 决策保持在原 Thread |
| C10 | `space::case_c10_*` | 持久化往返保留 Thread ID、版本、布局，重启将进行中的 View 标为 error |

运行方式：

```bash
cd prototype
pnpm test
cargo test --lib --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings
```

浏览器冒烟还会启动本地 bridge，发送一条只读请求，确认 SSE 状态进入 running rail、点击后打开执行详情，并在回合结束后恢复为空闲态。
