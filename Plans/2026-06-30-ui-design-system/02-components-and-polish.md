# UI 规范 · 组件模式与美化方向

## 1. 组件分层

```text
layout/     Sidebar, DevicesPanel
pages/      ClipboardPage, DevicesPage, SettingsPage
overlays/   PairingModal, IncomingConnectionPrompt, StatusNotice,
            TransferProgressCard, ConnectionAttemptCard, …
common/     SettingToggle, StatusDot, ThemeSwatch, …
```

新 UI 优先扩展 `common/`，避免在 page 内复制 class 串。

## 2. 标准组件配方（class 契约）

实施时抽成小组件或 `ui/*.tsx`（命名可选），以下为**目标 class 组合**。

### 2.1 `SurfaceCard`（列表/设置区块）

```text
overflow-hidden rounded-xl border border-border bg-card
transition-colors hover:border-primary/30   // 仅可点击时
```

内层嵌套块：

```text
rounded-xl border border-border bg-secondary/30 p-3
```

### 2.2 `PrimaryButton`

```text
rounded-lg bg-primary px-4 py-2.5 text-sm font-medium text-primary-foreground
transition-opacity hover:opacity-90 disabled:opacity-40 disabled:cursor-not-allowed
```

加载态：内容替换为 `Loader2` + `animate-spin`，宽度用 `min-w-[…]` 防抖动。

### 2.3 `SecondaryButton` / 图标按钮

```text
rounded-lg p-1.5 text-secondary-foreground
transition-colors hover:bg-secondary hover:text-foreground
disabled:opacity-40
```

方形图标按钮行内：`h-[26px] w-[26px]` 统一为 **`h-7 w-7`**（28px）或 `size-7`。

### 2.4 `TextField`

```text
rounded-lg border border-border bg-secondary px-3 py-2.5 text-sm text-foreground
placeholder:text-muted-foreground/60
transition-colors focus:border-primary focus:outline-none
disabled:opacity-50
```

配对码输入保留 `font-mono text-base tracking-[0.2em] text-center`。

### 2.5 `ModalShell`

```text
遮罩: fixed inset-0 z-* flex items-center justify-center
      absolute inset-0 bg-black/60 backdrop-blur-sm
面板: relative mx-4 w-full max-w-[380px] overflow-hidden
      rounded-2xl border border-border bg-card shadow-2xl
```

入站请求可在 header 保留 `border-primary/25 bg-primary/5` 以区分「需决策」。

### 2.6 `PageHeader`

各页顶栏统一结构：

```text
flex items-center justify-between gap-4 border-b border-border px-6 py-4
标题: text-base font-semibold text-foreground
副状态: text-xs font-medium text-muted-foreground
```

### 2.7 `StatusDot` / 连接状态

沿用 `StatusDot`；颜色与 `AppConnectionStatus` 映射保持一处定义（避免 page 内重复绿/黄/灰）。

### 2.8 `SettingRow`

设置页：`flex items-center justify-between gap-4 py-3` + 左侧标题 `text-sm font-medium` + 右侧控件。

## 3. 信息架构与布局美化

### 3.1 三栏剪贴板视图

- 左：剪贴板历史（主）
- 右：`DevicesPanel`（窄栏摘要）
- 美化：右栏与主区之间已有 border；可为右栏标题加 `text-xs uppercase tracking-wide text-muted-foreground`（可选，保持克制）

### 3.2 设备页

- 分区：发现 / 已连接 / 已信任 — 每区 `SurfaceCard` 包裹
- 「连接新设备」主 CTA 保持右下或标题区 `PrimaryButton` + 图标 `PlugZap`
- 刷新：`RefreshCw` + `animate-spin` 仅在 `isRefreshingDevices`（已有）

### 3.3 设置页

- 背景模式 + 主题色：`ThemeSwatch` 与侧栏底部重复 — **长期可只保留一处**（设置页为完整版，侧栏保留快捷切换或缩略）
- 分组标题：`text-xs font-semibold text-muted-foreground` + `mb-3`

### 3.4 侧栏导航

- 选中：`bg-secondary text-foreground`（或 `bg-primary/10 text-primary` 二选一，建议 **secondary 底** 减少刺眼）
- 未选中：`text-secondary-foreground hover:bg-secondary/50`
- 设备 nav 无文字 label 时靠图标 + tooltip（若加 tooltip 用同一套 opacity 过渡）

## 4. 现状差异清单（待收敛）

| 项 | 现状 | 目标 |
|----|------|------|
| 模态 z-index | 50 / 60 混用 | 按 01 文档栈 |
| 模态宽度 | 360 / sm | `max-w-[380px]` |
| 圆角碎片 | `rounded-[5.5px]` | `rounded-md` |
| 字号碎片 | 10/11/13 arbitrary | xs / sm / base |
| 主按钮 hover | opacity vs bg | 统一 opacity |
| PairingModal a11y | 缺 dialog 角色 | 对齐 IncomingConnectionPrompt |
| theme.css | 未使用 | 合并或删除 |

## 5. 视觉美化（不改结构）

1. **背景微渐变（可选）**：`.dark body` 极弱径向渐变 `primary/5` → transparent，避免纯平 `#090c14`；亮色同理 `primary/3`。
2. **卡片内分隔**：长列表项之间 `border-b border-border last:border-0` 代替双 border 卡片嵌套。
3. **焦点可见性**：`:focus-visible:ring-2 ring-ring ring-offset-2 ring-offset-background` 加到按钮与 nav（键盘用户）。
4. **空状态**：统一插图区 `rounded-xl border border-dashed border-border py-10 text-center text-sm text-muted-foreground`（配对列表空态已有，剪贴板历史空态对齐）。
5. **传输/连接卡**：`BottomRightStatusCard` 与 `TransferProgressCard` 共用外框样式 + 统一右下角 `right-6 bottom-6`；多卡同时出现时 **垂直堆叠 gap-3**（需在 App 层编排，避免重叠）。

## 6. Figma 对齐方式

- 大改版：读 Figma Design 节点对照 `PageHeader`、设置页分组间距
- 日常：以本文档 class 契约为准，Figma 作参考；改 token 时同步 Figma 变量（手动）

## 7. 不建议做的事

- 不引入 shadcn 全量（可按需抄单个 primitive）
- 不上 framer-motion 除非列表 stagger / 复杂手势（见 03-motion）
- 不把剪贴板内容区改成玻璃态 heavy blur
