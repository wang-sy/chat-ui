# Space — Product Specification

**Status:** Draft v0.1  
**更新时间:** 2026-10-11  
**当前阶段:** 概念验证 / UI 原型

## 1. 产品命题

Space 是一个建立在 Codex 执行能力之上的可视化工作空间。

用户不再面对一条不断增长的 chatbot 消息流，而是和一个 Main Thread 共同维护一个会持续变化的网页空间。Main Thread 负责理解用户意图、调度其他 Thread，并根据当前任务动态生成最合适的交互形态。

每个子 Thread 的交付物不是一段文字，而是一个可以运行的网页 View。Main Thread 可以把这些 View 折叠、展开、移动、放大、连接、组合或重新组织。

> Codex 负责理解和执行，Main Thread 负责组织空间，View 负责让工作变得可见。

## 2. 核心关系

```text
Space Runtime
└── Main Page
    └── Main Thread
        ├── View Page A ← View Thread A
        ├── View Page B ← View Thread B
        └── View Page C ← View Thread C
```

第一版采用以下关系：

- 一个 Space 只有一个 Main Thread；
- 一个 View 默认只由一个 View Thread 负责；
- Main Thread 可以创建、调度、暂停、恢复和关闭 View Thread；
- Main Thread 管理所有 View 的布局、关系和显示状态；
- View Thread 管理自己的网页内容、局部状态和局部交互；
- 用户默认和 Main Thread 交流，通过主页面影响 View Thread；
- 用户可以在需要时进入某个 View 的局部上下文。

## 3. 核心概念

### 3.1 Space

Space 是项目级的持续工作容器，保存：

- 一个 Main Thread；
- 当前主页面；
- View 注册表；
- Thread 和 View 的关系；
- 事件、决策和版本；
- 共享上下文和产出物。

Space 不是一个固定的 dashboard。它只提供最小运行时，让 Main Thread 可以生成和组织页面。

### 3.2 Main Thread

Main Thread 是人与系统交流的主要入口，也是全局协调者。

它负责：

- 接收并理解用户目标；
- 判断需要哪些 View 和 View Thread；
- 给 View Thread 分配范围明确的任务；
- 读取各个 View 的状态和结果；
- 生成和修改主页面；
- 调整 View 的位置、大小、层级和关系；
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

iframe 是第一版的隔离和承载方式。更本质的概念是“由 Thread 生成并由 Space 组合的网页 View”。

### 3.5 Main Page

Main Page 由 Main Thread 管理。它不是一个预先固定的页面模板，而是根据用户当前的目标和交互不断变化的网页。

Main Thread 可以让 Main Page 呈现为：

- 多个 View 的画布；
- 左右对照空间；
- 流程和证据的关系图；
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
Main Thread 重新组织 Main Page
    ↓
用户通过页面观察、选择和操作
    ↓
操作被路由给 Main Thread 或对应 View Thread
    ↓
继续执行
```

用户不需要逐条阅读所有 Thread 的文字输出。默认只需要看到：

- 当前空间正在做什么；
- 哪些 View 有变化；
- 哪些 View 正在工作；
- 哪些地方需要用户判断；
- 当前操作会影响哪个 Thread。

## 5. 权限和所有权

### Main Thread 可以做的事

- 创建和销毁 View；
- 加载和卸载 View；
- 移动、缩放、折叠和展开 View；
- 调整 View 的层级和布局；
- 在 View 之间建立连接；
- 聚焦或隐藏某个 View；
- 请求 View Thread 更新自己的内容；
- 生成新的主页面交互。

### View Thread 可以做的事

- 修改自己的网页内容和局部状态；
- 报告自己的工作状态；
- 请求用户或 Main Thread 的判断；
- 提交文件、图表、代码或其他产出物；
- 请求 Main Page 聚焦自己的 View；
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

space.mount_view
space.resize_view
space.collapse_view
space.expand_view
space.connect_views
space.focus_view
space.route_action
```

iframe 之间不直接通信。所有跨 View 的关系和操作都通过 Space Runtime 与 Main Thread 进行路由。

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

1. **Space 是动态网页，不是固定 dashboard。**
2. **Main Thread 管理全局组合，View Thread 管理局部网页。**
3. **每个视觉状态都应该可以追溯到 Thread、文件、事件或决定。**
4. **View 的内部实现可以复杂，Space 的全局交互必须保持简单。**
5. **用户通过观察和操作空间推进工作，不需要逐条阅读模型输出。**
6. **所有重要变化都应当可比较、可确认、可撤销。**
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

## 10. 第一版原型范围

第一版只验证空间关系和交互方式：

- 一个 Space；
- 一个 Main Thread；
- 一个由 Main Thread 管理的主画板；
- 三个 View Thread；
- 三个独立网页 View：系统架构、支付流程、调用时序；
- View 可以被移动、折叠、展开和聚焦；
- Main Thread 可以在 View 之间绘制连接；
- 底部只有一个向 Main Thread 输入意图的入口；
- 使用模拟状态展示 Thread 同时工作；
- 暂时不接真实 Codex API，先验证信息架构和交互感觉。

## 11. 需要继续决定的问题

- Main Page 是否允许完全由 Main Thread 重写，还是保留一个极薄的运行时壳；
- View 的布局是否完全自由，还是保留少量可预测的布局约束；
- 用户是否可以直接对某个 View Thread 输入；
- View 是否可以在不创建 Thread 的情况下存在；
- iframe 之外是否需要支持同源页面或 Web Component；
- Space 的状态和 View 文件如何持久化；
- Main Thread 如何判断应该创建 View、更新 View，还是直接回复用户。
