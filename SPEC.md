# Chat UI — Product Specification

**Status:** Draft v0.1  
**更新时间:** 2026-10-11  
**当前阶段:** 概念验证 / UI 原型

## 1. 产品命题

Chat UI 是一个建立在 Codex 执行能力之上的可视化工作空间。

用户不再面对一条不断增长的 chatbot 消息流，而是和一个 Main Thread 共同维护一个会持续变化的网页空间。Main Thread 负责理解用户意图、调度其他 Thread，并根据当前任务动态生成最合适的交互形态。

每个子 Thread 交付适合任务的交互式可视化。架构、流程和关系类任务优先使用 SVG 图；HTML 是隔离运行的载体，不要求结果长成介绍网页。Main Thread 负责把真实的子 View 内嵌进主页面，并展示它们之间的数据流、依赖、证据和反馈关系。

主、子 Thread 都默认先考虑图形表达，不依赖用户说出“画图”或“SVG”。“总览、工作台、页面”只说明承载位置：理解关系用结构图，理解先后用时序/流程，理解变化用状态图，理解数量用图表，方案对照用同尺度比较。确实需要录入、编辑和操作时保留表单或编辑器；一句话能回答的问题不额外创建 View。

> Codex 负责理解和执行，Main Thread 负责组织空间，View 负责让工作变得可见。

## 2. 核心关系

```text
Chat UI Runtime
└── Main Page
    └── Main Thread
        ├── View Page A ← View Thread A
        ├── View Page B ← View Thread B
        └── View Page C ← View Thread C
```

第一版采用以下关系：

- 一个 Chat UI 只有一个 Main Thread；
- 一个 View 默认只由一个 View Thread 负责；
- Main Thread 可以创建、调度、暂停、恢复和关闭 View Thread；
- Main Thread 管理 View 注册表、关系、版本和显示状态；
- View Thread 管理自己的网页内容、局部状态和局部交互；
- 用户默认和 Main Thread 交流，通过主页面影响 View Thread；
- 用户可以在需要时进入某个 View 的局部上下文。

## 3. 核心概念

### 3.1 Chat UI

Chat UI 是项目级的持续工作容器，保存：

- 一个 Main Thread；
- 当前主页面；
- View 注册表；
- Thread 和 View 的关系；
- 事件、决策和版本；
- 共享上下文和产出物。

Chat UI 不是一个固定的 dashboard。它只提供最小运行时，让 Main Thread 可以生成和组织页面。

### 3.2 Main Thread

Main Thread 是人与系统交流的主要入口，也是全局协调者。

它负责：

- 接收并理解用户目标；
- 判断需要哪些 View 和 View Thread；
- 给 View Thread 分配范围明确的任务；
- 读取各个 View 的状态和结果；
- 生成和修改主页面；
- 管理 View 的版本、关系和生命周期；
- 在关键节点向用户请求判断；
- 将用户在主页面上的操作路由给正确的 Thread。

Main Thread 不直接读取或修改子 View 的内部 DOM。它通过公开的状态和事件协议管理 View。

### 3.3 View Thread

View Thread 是一个有明确工作范围的 Codex Thread。它负责完成一个局部任务，并交付一个可运行的 View 页面。

它负责：

- 读取被分配的上下文和文件；
- 理解局部目标；
- 编写和修改自己的 HTML、CSS、JavaScript 或其他产出物；
- 在自己的 View 中显示局部工作状态；
- 报告进度、变化、错误和待决策事项；
- 根据 Main Thread 或用户的反馈继续迭代。

View Thread 默认不直接修改其他 View，也不直接控制主页面布局。

### 3.4 View

View 是 Thread 维护的可运行网页表面，不是静态截图，也不只是一个 HTML 文件。

一个 View 包含：

- 结构和视觉状态；
- 局部交互逻辑；
- 当前版本；
- 来源 Thread；
- 状态和错误信息；
- 可以被 Main Thread 调用的事件和能力。

View 可以呈现为：

- 系统架构图；
- 用户流程图；
- 时序图；
- 时间线；
- 代码预览；
- 数据分析；
- 对照方案；
- 任意适合当前任务的网页交互形态。

iframe 是第一版的隔离和承载方式。更本质的概念是“由 Thread 生成并由 Chat UI 组合的网页 View”。

### 3.5 Main Page

Main Page 由 Main Thread 管理，默认职责是把 Space 内所有保留的 View 以真实 iframe 组合展示。宿主只加载内容，不强制添加标题栏、边框、状态或折叠外壳。Main 自主设计布局、顺序、对照、关系和交互；默认直接展示结果，需要折叠时自行实现并保留页面状态，不应另做一个只有导航入口的网站。

Main Thread 可以让 Main Page 呈现为：

- 多个 View 内容直接内嵌的结果总览；
- 左右对照页面；
- 流程和证据的页面化呈现；
- 任务和代码的组合工作台；
- 一个需要用户逐步操作的界面；
- 任何能更贴切地表达当前任务的形态。

## 4. 核心交互循环

```text
用户表达意图
    ↓
Main Thread 理解全局目标
    ↓
Main Thread 创建或调度 View Thread
    ↓
View Thread 读取上下文并生成 / 修改 View
    ↓
View 报告状态和变化
    ↓
Main Thread 更新 Main Page 和 View 注册表
    ↓
用户通过页面观察、选择和操作
    ↓
操作被路由给 Main Thread 或对应 View Thread
    ↓
继续执行
```

用户不需要逐条阅读所有 Thread 的文字输出。默认只需要看到：

- 当前工作区正在做什么；
- 哪些 View 有变化；
- 哪些 View 正在工作；
- 哪些地方需要用户判断；
- 当前操作会影响哪个 Thread。

### 4.1 运行中的对话队列

Thread 执行期间，输入框仍然保持可用。用户继续发送的消息进入对应 Main 或 View Thread 的有序队列，当前 turn 结束后自动按顺序发送；队列保存每条消息发送时的模型、推理强度、速度和权限快照，并在刷新后恢复。队列中的消息支持引导回输入框、编辑、删除和拖动或键盘排序。停止当前 turn 不会丢弃队列，线程恢复空闲后继续处理。

## 5. 权限和所有权

### Main Thread 可以做的事

- 创建和销毁 View；
- 加载和卸载 View；
- 管理 View 的版本、关系和生命周期；
- 聚焦某个 View Thread 并切换其页面；
- 请求 View Thread 更新自己的内容；
- 生成新的主页面交互。

### View Thread 可以做的事

- 修改自己的网页内容和局部状态；
- 报告自己的工作状态；
- 请求用户或 Main Thread 的判断；
- 提交文件、图表、代码或其他产出物；
- 请求用户打开自己的 View Thread；
- 提议和其他 View 建立关系。

### View Thread 不应该做的事

- 直接修改其他 View 的 DOM；
- 直接重排 Main Page；
- 悄悄修改共享状态；
- 把不可追溯的图片当作唯一事实来源。

## 6. View 与 Main Page 的通信

第一版采用结构化事件协议。事件名称可以先保持最小：

```text
view.ready
view.status_changed
view.updated
view.needs_decision
view.artifact_created
view.request_focus
view.error

chat_ui.mount_view
chat_ui.resize_view
chat_ui.collapse_view
chat_ui.expand_view
chat_ui.connect_views
chat_ui.focus_view
chat_ui.route_action
```

iframe 之间不直接通信。所有跨 View 的关系和操作都通过 Chat UI Runtime 与 Main Thread 进行路由。

## 7. 结构化输出

每次 Thread 更新都应该能被转换成结构化状态，而不是只有一段自然语言：

```text
summary       当前发生了什么
state_delta   哪些状态发生了变化
proposal      建议修改什么
question      需要人决定什么
artifact      产生了什么网页、文件或结果
evidence      修改依据来自哪里
```

文字是解释层，View 和结构化状态是工作层。

## 8. 设计原则

1. **Chat UI 是动态网页，不是固定 dashboard。**
2. **Main Thread 管理全局组合，View Thread 管理局部网页。**
3. **每个视觉状态都应该可以追溯到 Thread、文件、事件或决定。**
4. **View 的内部实现可以复杂，Chat UI 的全局交互必须保持简单。**
5. **用户通过观察和操作空间推进工作，不需要逐条阅读模型输出。**
6. **所有重要变化都应当可比较、可确认，并保留清晰的版本边界。**
7. **iframe 是实现手段，网页 View 才是产品对象。**

## 9. 非目标

第一版不做：

- 新的模型训练或模型内核；
- 复杂的 agent 市场；
- 多 agent 的自由编排器；
- 固定的 chatbot 消息流作为主界面；
- 让 Main Thread 直接接管所有子 View 的内部代码；
- 为每个简单状态都创建一个完整 iframe；
- 展示模型内部的完整隐式推理过程。

## 10. 第一版视觉原型范围

第一版只验证空间关系和交互方式：

- 一个 Chat UI；
- 一个 Main Thread；
- 一个由 Main Thread 管理的主页面；
- 三个 View Thread；
- 三个独立网页 View：系统架构、支付流程、调用时序；
- 用户可以从 Space 树进入任意 View Thread 页面；
- 底部只有一个向 Main Thread 输入意图的入口；
- 视觉原型使用真实的本机 Codex 连接状态；View 编排和网页产出仍按后续阶段逐步接入。

## 11. 需要继续决定的问题

- Main Page 允许由 Main Thread 自由重写，Chat UI Runtime 只保留一个极薄的运行时壳；
- View 之间的关系元数据是否需要继续对用户可见；
- 用户是否可以直接对某个 View Thread 输入；
- View 是否可以在不创建 Thread 的情况下存在；
- iframe 之外是否需要支持同源页面或 Web Component；
- Chat UI 的状态和 View 文件落在项目内的 `.space/` 目录；`.space/` 是当前版本的持久化目录名。
- Main Thread 如何判断应该创建 View、更新 View，还是直接回复用户。

## 12. 实现基线

当前实现采用跨平台优先的本地服务架构：

```text
本地 Chat UI Service（Rust）: 127.0.0.1:4317
  ├── 本机 Codex app-server：单一共享线程运行时
  ├── HTTP JSON API：浏览器端和 Tauri 端共用
  ├── 项目目录与 .space/chat-ui.json：线程消息和项目状态
  └── React + TypeScript + Vite：Codex 风格的界面
Tauri 2
  └── 启动本地服务并承载 React UI
```

实现约束：

- 第一版同时考虑 Windows、macOS 和 Linux 的 WebView 差异；
- Chat UI 只使用用户本机已有的 Codex，不把 Codex runtime 直接打进安装包；
- Rust 服务启动后自动扫描当前进程的 `PATH`，查找 `codex`、Windows 下的 `codex.exe`、`codex.cmd` 或 `codex.bat`；找不到时在界面提示用户安装或配置 PATH，不要求填写可执行文件路径；
- 一个本地服务只管理一个 Codex app-server 子进程。浏览器和 Tauri UI 连接同一个服务，因此看到同一组项目、聊天和活动状态；
- Tauri 启动时自动拉起服务；浏览器开发命令 `pnpm dev` 在需要时拉起同一个服务。关闭 UI 时只结束由当前开发进程启动的服务；
- 项目清单保存在用户目录的 `.chat-ui/projects.json`，每个项目的聊天状态保存在项目内 `.space/chat-ui.json`，便于透明查看和 Git 管理；
- 宿主只提供目录选择、线程生命周期、权限请求和受控 Bridge，不把 Main Page 固定成 dashboard，也不渲染额外的节点画布；
- Main Thread 和 View Thread 已通过结构化 turn 输出接入编排协议：Main Thread 返回完整的 Space plan（`mainPage`、`views`、`connections`、显式的 `removeViews`），运行时创建或复用 View Thread；View Thread 返回受限的页面结果，运行时只更新自己的 View。主页面和 View 页面分别落在 `.space/main/` 与 `.space/views/<view-id>/`，并继续由 Space 注册表统一管理。

## 13. 本地线程与服务协议

本地服务监听 `127.0.0.1:4317`，启动后通过 `GET /health` 返回一次性服务 token。浏览器端和 Tauri 端都使用 `X-Chat-UI-Token` 访问以下接口：

| 接口 | 作用 |
| --- | --- |
| `GET /api/state` | 返回连接状态、模型、项目、聊天和活动请求 |
| `POST /api/connect` | 自动发现 PATH 中的 Codex 并启动 app-server |
| `GET /api/events` | 带序号的 SSE 事件流，转发 Codex 生命周期、文本增量、计划、工具和审批状态 |
| `GET /api/directories` | 浏览本机目录，供 landing page 选择项目 |
| `POST /api/projects` | 添加或选中项目目录 |
| `POST /api/chats/send` | 创建或恢复线程并开始一轮 Codex turn |
| `POST /api/chats/steer` | 使用 `turn/steer` 向活动 turn 追加一条消息 |
| `POST /api/chats/open` | 恢复已有线程和消息 |
| `POST /api/chats/interrupt` | 中断正在运行的 turn |
| `POST /api/requests/reply` | 回复 Codex 的权限确认或用户输入请求 |

线程生命周期为：选择目录 → 首次发送时调用 `thread/start` → 后续消息复用同一个 `threadId` → 服务转发 app-server 的增量消息和请求 → 运行中发送进入队列，明确点击引导时调用 `turn/steer` → SSE 事件流同步共享状态。Tauri 和浏览器共享 Codex 线程状态，轮询仅作为状态同步兜底，不能覆盖更新的流式事件。

展开每个 Main / View Thread 的对话框时，展示该线程的历史消息及执行记录。回复、公开思考摘要、命令输出、工具进度和计划随事件累积更新；完成后仍可展开检查，刷新后恢复。结构化结果只展示面向用户的 `response`，页面 HTML 与内部编排提示不进入对话。折叠后保留紧凑输入框与运行状态条；查看旧消息时新输出不能强制抢走滚动位置。

## 14. 图式交付与结果组合

- Main Page 是完整的可视化结果，不能只列卡片、标题和跳转链接。Main 负责展示整体结构、关系方向和关系含义；子 View 负责局部图和交互。
- Main 为所有保留和新建的 View 使用 `<chat-view view-id="architecture"></chat-view>` 声明嵌入位置，每个 View 出现一次，默认直接展示。引用可用真实 View ID 或持久化的语义 key。宿主只提供无边框的真实子页面、自动高度、独立更新及事件路由，不添加标题栏、“已就绪”、折叠和跳转控件。Main 自主设计排版、标题、样式和按需折叠，不复制、伪造或重写子 View 的内部 HTML，也不以目录取代真实结果。
- 每块 iframe 默认按实际内容调整高度，Main 可通过 CSS 或 `--chat-view-height` 指定适合场景的视口；长页面可滚动浏览。Main 自行提供折叠时保留 iframe 和交互状态，宿主不强制展开或覆盖布局。旧 Main 页面或局部计划遗漏 View 时，组合运行时补齐真实内嵌，不移动、折叠或改写既有主页面内容。
- 子 View 独立加载、失败、重试和更新；一个 View 更新不应重新加载其他已挂载 View。生成中的 View 显示真实状态，不能显示伪造完成内容。
- `connections` 保存带方向和语义标签的关系；Main 同时用 SVG、流程图或对照结构把这些关系呈现出来。子结果返回后，Main 进行一次受限的整合，依据实际摘要补充结论，不重复分派子任务。
- 架构、流程、依赖和时序任务以可读 SVG 为主，展示节点、连线、箭头和标签；节点选择提供局部说明。指标任务以图表及必要数据为主。避免大标题、营销文案和只有文字的卡片墙。
- 页面内普通选择和联动不启动 Agent。子 View 可发出 `selection` 事件，携带共同的 `entityId`；宿主组合层向同一 Main Page 的其他 View 转发上下文。明确需要 Agent 改动时才使用带 `intent` 的 action。
- Main 可用原生 `<details>` 或自定义可访问控件实现按需折叠；单独打开 View 也是可选的，由 Main 使用 `data-chat-open-view` 或已知 View 的导航链接声明。宿主不强制提供这些按钮，未知链接不会猜测或打开其他 Space。
- 每个 iframe 保持脚本/表单沙箱，不获得本地服务 token、文件访问权限或其他 View 的 DOM。

验收：推荐系统 Main Page 同时显示架构图和指标图，并标注推荐→曝光/点击→指标→优化反馈；点击架构节点可看到解释，并联动指标关注状态；修改指标 View 后，主页面嵌入内容更新，架构图的局部选择保持。整合阶段不会创建重复 View 或触发无限重跑。

选型验收另用 10 个只描述用户目标的真实场景，预期形式不提供给 Agent。检查 Main 的任务分配、View 的实际图形语义、嵌入后的可读性和交互，不能以出现 `<svg>` 或生成成功代替效果验收。包含录入表单和简短问答两个对照场景，避免机械地把所有请求变成图。
