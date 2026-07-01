# App.tsx 拆分重构 — 待执行方案

> 生成时间: 2026-06-22
> 基于讨论: [01-app-tsx-splitting-discussion.md](01-app-tsx-splitting-discussion.md) | [02-app-splitting-boundary-and-order.md](02-app-splitting-boundary-and-order.md)

## 需求概述

当前 `Apps/planarclip/src/app/App.tsx` 接近 2000 行，单文件同时承载了类型定义、常量配置、纯工具函数、通用 UI 组件、页面级组件、Tauri 桥接逻辑、配对流程状态与顶层页面组装职责。该结构虽然可运行，但已经显著影响可维护性、可读性和后续迭代效率。

本主题目标不是“机械地把大文件切碎”，而是在**不改变现有行为**的前提下，将 `App.tsx` 重构为**职责明确的顶层容器文件**，把类型、常量、工具函数和视图层逐步拆出，并为后续进一步抽离连接桥接与配对流程 hooks 做准备。

本轮主题聚焦于 **App.tsx 拆分与结构收敛**，不把测试基线建设、全局状态管理升级或额外产品能力扩展混入主线。

## 技术决策

| 决策项 | 选择 | 理由 | 来源轮次 |
|--------|------|------|----------|
| 重构目标 | 先做职责收敛，不追求一次性极致架构化 | 当前主要问题是职责混杂，优先提升可维护性并控制回归风险 | 第 1 轮 |
| 第一阶段策略 | 先拆类型/常量/纯函数/组件，不改变状态流 | 这类内容边界清晰、搬家成本低，适合先做无行为变化重构 | 第 1 轮、第 2 轮 |
| 第二阶段策略 | 再评估抽 `useUiTheme / useConnectionBridge / usePairingFlow` | 当前连接桥接和配对流程强耦合，过早抽 hook 容易让行为回退 | 第 1 轮、第 2 轮 |
| 状态管理方案 | 暂不引入 Zustand / Redux / Context 升级 | 当前主题聚焦单文件拆分，避免改动面扩散与目标失焦 | 第 2 轮 |
| 组件拆分原则 | 页面级组件与通用组件分别建目录 | 区分复用型组件与业务区块，避免新目录再次混杂 | 第 1 轮、第 2 轮 |
| 验证策略 | 以 `pnpm check:web`、`pnpm build:web`、`pnpm check` + 必要手工回归为主 | 本轮是结构重构，优先采用轻量且高覆盖的验证链路 | 第 2 轮 |
| 遗留事项处理 | `vp test` 继续保留在本主题 `UNEXECUTED.md`，暂不纳入主线 | 避免结构重构与测试基线建设相互干扰 | 第 1 轮 |

## 架构设计

### 目标目录结构

```text
src/app/
  App.tsx
  types.ts
  constants/
    theme.ts
  utils/
    clipboard.ts
    device.ts
    message.ts
    settings.ts
    time.ts
  hooks/
    useRelativeTicker.ts
    useUiTheme.ts
    useConnectionBridge.ts
    usePairingFlow.ts
  components/
    common/
      OsIcon.tsx
      StatusDot.tsx
      ClipTypeIcon.tsx
      CopyButton.tsx
      SettingBadge.tsx
      ThemeSwatch.tsx
    layout/
      Sidebar.tsx
      DevicesPanel.tsx
    pages/
      ClipboardPage.tsx
      DevicesPage.tsx
      SettingsPage.tsx
    overlays/
      PairingModal.tsx
```

### 责任分层

#### 1. 模型与常量层
- `types.ts`：承载领域类型、payload 类型、UI 相关联合类型
- `constants/theme.ts`：承载主题色常量、默认设置、主题规范化方法

#### 2. 工具函数层
- `utils/clipboard.ts`：剪贴板历史映射、大小格式化
- `utils/device.ts`：设备 OS 推断、设备列表构建、设备 id 生成
- `utils/message.ts`：技术错误转用户中文提示
- `utils/settings.ts`：浏览器预览态设置读写、主题应用
- `utils/time.ts`：相对时间与显示时间格式化

#### 3. 视图层
- `components/common/*`：低层复用组件
- `components/layout/*`：侧栏与右侧概览栏等布局组件
- `components/pages/*`：剪贴板页、设备页、设置页
- `components/overlays/*`：弹层与浮层

#### 4. 顶层容器层
- `App.tsx`：仅负责顶层状态、事件处理协调、页面组装
- 后续再逐步把其中的主题逻辑、连接桥接、配对流程抽入 hooks

### 关键约束

- 第一轮拆分必须保持现有页面行为、命令调用和事件订阅行为一致
- 不为了减少代码行数而制造过度的 props drilling 或循环依赖
- 不把连接桥接逻辑在第一轮中切得过碎，避免调试路径恶化
- 不在本主题中顺手引入新状态库或新全局架构

## 实现步骤

### Phase 1：静态内容与基础复用层拆分
- [ ] 创建 `src/app/types.ts`，迁移当前 `App.tsx` 顶部的领域类型与 payload 类型
- [ ] 创建 `src/app/constants/theme.ts`，迁移主题色、默认设置及相关规范化函数
- [ ] 创建 `src/app/utils/time.ts`、`clipboard.ts`、`device.ts`、`message.ts`、`settings.ts`
- [ ] 创建 `src/app/hooks/useRelativeTicker.ts`，迁移相对时间刷新 hook
- [ ] 调整 `App.tsx` 导入路径，确保 Phase 1 结束后功能不变

### Phase 2：视图组件拆分
- [ ] 创建 `src/app/components/common/`，拆出 `OsIcon`、`StatusDot`、`ClipTypeIcon`、`CopyButton`、`SettingBadge`、`ThemeSwatch`
- [ ] 创建 `src/app/components/layout/Sidebar.tsx` 与 `DevicesPanel.tsx`
- [ ] 创建 `src/app/components/pages/ClipboardPage.tsx`、`DevicesPage.tsx`、`SettingsPage.tsx`
- [ ] 创建 `src/app/components/overlays/PairingModal.tsx`
- [ ] 保持现有 props 结构优先不变，避免第一轮同时重构状态流
- [ ] 收敛 `App.tsx` 为“状态 + handlers + 页面组装”结构

### Phase 3：逻辑收敛与后续抽象评估
- [ ] 评估 `App.tsx` 拆分后剩余体量与逻辑聚集度
- [ ] 视情况抽 `useUiTheme`，封装主题同步与设置持久化逻辑
- [ ] 视情况抽 `useConnectionBridge`，封装初始化、事件监听、状态桥接逻辑
- [ ] 评估 `usePairingFlow` 是否适合独立，避免把隐式状态机拆成半吊子结构
- [ ] 清理潜在循环依赖，必要时补统一导出入口

### Phase 4：验证与收尾
- [ ] 运行 `pnpm check:web`
- [ ] 运行 `pnpm build:web`
- [ ] 运行 `pnpm check`
- [ ] 重点回归启动初始化、主题切换、局域网连接、配对码流程、来连请求、连接失败/结束、剪贴板历史更新
- [ ] 如需要，在桌面端手工验证关键连接链路与配对弹层行为
- [ ] 更新 `COMPLETED.md` / `UNEXECUTED.md`，记录实际拆分范围与仍待继续收敛的事项

## 风险与回归点

### 高风险区域
- `App.tsx` 后半段的初始化与事件监听链路
- 配对流程相关状态机（`pairingStage`、`incomingRequest`、`pairingError` 等）
- 主题设置与预览态存储逻辑
- 设备列表构建与当前连接对象补全逻辑

### 必测清单
- 启动后是否仍能正确加载：状态、配对码、LAN 设备、UI 设置、剪贴板历史
- 切换背景模式和主题色后，桌面端 / 浏览器预览态行为是否正确
- 点击局域网设备 → 收到配对码需求 → 输入配对码 → 建立连接 → 断开连接
- 收到来连请求后，弹层展示与拒绝连接是否正常
- 连接失败与连接结束后的用户提示是否仍为自然中文
- 剪贴板历史列表 / 网格切换、空态与复制按钮是否仍可用
- `clipboard-history-changed` 事件触发后 UI 是否实时更新

## 本主题暂不纳入的内容

- 建立完整测试基线与补 `vp test`
- 引入 Zustand / Redux / Context 之类全局状态层重构
- 扩展新的产品能力或新页面
- 改造现有连接协议、Tauri command 接口或 Rust 后端逻辑

## 关键依赖

- React + TypeScript
- `@tauri-apps/api/core`
- `@tauri-apps/api/event`
- `lucide-react`
- 当前已有前端构建链：`pnpm check:web` / `pnpm build:web` / `pnpm check`

## 继承事项

- `在建立测试基线后补 vp test`
  - 当前不纳入本次 App.tsx 拆分主线
  - 继续保留在本主题 [UNEXECUTED.md](UNEXECUTED.md) 中跟踪

## 参考讨论

- [01-app-tsx-splitting-discussion.md](01-app-tsx-splitting-discussion.md)
- [02-app-splitting-boundary-and-order.md](02-app-splitting-boundary-and-order.md)
