# PlanarClip UI 与前端视觉规范

本文件是 PlanarClip **项目级 UI 规范**：用户可见文案与交互、设计令牌、组件模式、动画与浮层层级。编码代理与人类开发者做前端改动时均应遵循；更细的执行清单见 [`Plans/2026-06-30-ui-design-system/`](../Plans/2026-06-30-ui-design-system/execution-plan.md)。

**样式真源**：`Apps/src/styles/index.css`（CSS 变量 + Tailwind `@theme`）、`Apps/src/app/constants/theme.ts`（四套主题色）。Figma Design 可作为视觉对照，实现以代码令牌为准。

---

## 1. 设计原则

| 原则 | 说明 |
|------|------|
| 工具感优先 | 信息密度适中、操作就近；避免大面积装饰与过长动画 |
| 令牌单一来源 | 颜色、圆角、动效时长从 CSS 变量或本文约定 class 读取；组件内禁止硬编码主题 hex |
| 状态可感知 | 连接、配对、传输、错误四类流程使用统一视觉语言 |
| 中文界面 | 用户可见文案默认中文（见第 2 节） |
| 尊重系统 | 遵守 `prefers-reduced-motion`（见第 9 节） |

---

## 2. 用户可见文案

（自 [`AGENTS.md`](../AGENTS.md) 提取并固化。）

- 用户可见的界面文本、按钮文案、状态文案、成功提示、警告提示、错误提示，**默认全部使用中文**。
- 提示文案必须使用自然语言，优先描述：用户当前遇到的问题、系统正在做的事、用户下一步可以怎么做。
- **禁止**直接向用户暴露程序术语、底层异常、协议名、库报错、系统调用错误、英文错误原文，除非用户明确进入开发排障场景。
- 底层技术错误展示前须转换为自然语言；必要时附带简短建议（如检查网络、确认对端已打开、稍后重试）。
- 成功提示简洁明确，例如「已连接到设备」「配对成功」「已完成同步」；避免生硬工程术语。
- 警告与失败须说明原因和影响，例如「未能连接到对方设备，请确认对方应用已打开」；不要只显示「连接失败」或原始错误对象。
- 同一类状态提示口径一致：避免中英混用、术语混乱（例如一处「配对」、另一处「握手」）。
- 调试用的技术细节写入日志或开发者输出，**不要**放在用户界面。

---

## 3. 交互与按钮

（自 [`AGENTS.md`](../AGENTS.md) 提取并固化。）

- 刷新、增加、删除等简单操作优先 **纯图标按钮**。
- 能用图标准确表达时，默认不再附带文字，但必须提供 **悬浮提示（`title`）或 `aria-label`**。
- 连接、断开、刷新等高频动作放在 **列表项右侧或标题操作区**，保持就近与视觉统一。
- 主按钮加载态使用 `Loader2` + `animate-spin`；禁用时 `opacity-40` / `cursor-not-allowed`。
- 破坏性操作（断开等）使用 `destructive` 色系（如 `bg-destructive/12`、`text-destructive`），与主操作区分。
- **背景 hover / focus** 须使用 **Surface Reveal**（见第 9.4 节），禁止在按钮、输入、导航、开关上用 `transition-colors` / `opacity` 替代主反馈。

---

## 4. 色彩与表面层级

### 4.1 语义表面（暗色为默认体验）

| 令牌 | 用途 |
|------|------|
| `background` | 应用画布 |
| `card` | 侧栏、主卡片、弹层面板 |
| `secondary` | 嵌套块、输入底、列表项底 |
| `muted` / `muted-foreground` | 分隔区、说明文案 |
| `primary` / `ring` | 主操作、焦点、品牌强调（随 `theme_color` 切换） |
| `destructive` | 断开、拒绝、超时紧迫态 |
| `border` | 统一描边 |

主题色四套定义在 `constants/theme.ts`（`THEME_COLORS`），由 `useUiTheme` 写入 `--primary`、`--ring`、`--accent` 等。

### 4.2 扩展语义（推荐）

| 变量 | 用途 | 实现建议 |
|------|------|----------|
| success | 配对成功、同步完成 | 映射 `--chart-2` |
| warning | 即将超时 | 映射 `--chart-4` |
| overlay | 模态遮罩 | 与 `bg-black/60` 一致 |

---

## 5. 圆角、间距、阴影

### 5.1 圆角（`--radius` 默认 8px）

| Tailwind | 用途 |
|----------|------|
| `rounded-md` | 图标按钮、小输入、侧栏 logo 块 |
| `rounded-lg` | 默认按钮、输入框、导航项 |
| `rounded-xl` | **标准卡片**、设备行内块 |
| `rounded-2xl` | **模态面板**、底部 Toast、传输卡 |
| `rounded-full` | 开关、状态点、进度条轨道 |

避免任意值圆角（如 `rounded-[5.5px]`），新代码一律用上表。

### 5.2 间距

仅使用 Tailwind 刻度：`1` `1.5` `2` `2.5` `3` `4` `5` `6`。区块纵距默认 `space-y-4` / `gap-4`；卡片内 `p-3` 或 `p-4`；弹层节间距 `space-y-5`。

页面水平内边距：剪贴板 / 设备 / 设置页对齐（推荐 `px-6 py-4` 页头区）。

### 5.3 边框与 hover

- 标准卡片：`border border-border bg-card`
- 可点击卡片：`app-surface-reveal-bg` + `hover:border-primary/30`（边框与背景同属 surface reveal，见第 9.4 节）；仅改边框、不改背景时仍可用 `transition-colors`，但新代码优先与 surface reveal 对齐
- 主按钮 hover：使用 **`app-surface-reveal-bg`** + `hover:bg-[var(--button-primary-hover-bg)]`（勿再用整颗 `hover:opacity-90` 作为主反馈）

### 5.4 阴影

- 模态 / Toast：`shadow-2xl`
- 遮罩：`backdrop-blur-sm` + 半透明黑底
- 避免全局 heavy glass / 大 blur

---

## 6. 排版与图标

### 6.1 字体

- 界面：`Plus Jakarta Sans` + 中文回退（`index.css` `body`）
- 配对码、地址：`font-mono`（JetBrains Mono）
- 正文不引入第三种字体

### 6.2 字号阶梯

| 角色 | class | 用途 |
|------|-------|------|
| 页面标题 | `text-base font-semibold` | 各页顶栏 |
| 正文 | `text-sm` | 列表、按钮 |
| 辅助 / 元数据 | `text-xs font-medium` | 时间、状态、section 标签 |
| 配对码输入 | `font-mono text-base tracking-[0.2em]` | 六位码 |

新代码避免 `text-[10px]` / `text-[11px]` 等碎片；存量逐步收敛到 `xs` / `sm` / `base`。

### 6.3 图标（lucide-react）

- 导航 / 侧栏：`size={15}`
- 行内操作：`14`
- 模态标题区：`15`–`18`

---

## 7. 组件分层与 class 契约

```text
app/components/
  layout/     Sidebar, DevicesPanel
  pages/      ClipboardPage, DevicesPage, SettingsPage
  overlays/   PairingModal, IncomingConnectionPrompt, StatusNotice, …
  common/     SettingToggle, StatusDot, ThemeSwatch, …
```

优先在 `common/` 或未来的 `ui/` 复用下列模式，避免在 page 内复制长 class 串。

| 模式 | 核心 class |
|------|------------|
| **SurfaceCard** | `overflow-hidden rounded-xl border border-border bg-card`（可点击加 hover border） |
| **嵌套块** | `rounded-xl border border-border bg-secondary/30 p-3` |
| **PrimaryButton** | `rounded-lg bg-primary … app-surface-reveal-bg hover:bg-[var(--button-primary-hover-bg)]`（见 `components/ui/PrimaryButton.tsx`） |
| **IconButton** | `app-surface-reveal-bg hover:bg-secondary hover:text-foreground …`（见 `components/ui/IconButton.tsx`） |
| **TextField** | `rounded-lg border border-border bg-secondary px-3 py-2.5 text-sm … focus:border-primary focus:outline-none` |
| **ModalShell** | 全屏 flex 居中；遮罩 `bg-black/60 backdrop-blur-sm`；面板 `relative mx-4 w-full max-w-[380px] rounded-2xl border border-border bg-card shadow-2xl` |
| **PageHeader** | `flex items-center justify-between gap-4 border-b border-border px-6 py-4` |
| **ScrollArea** | 根节点 `app-scrollbar` + `components/ui/ScrollArea` + `useScrollbarReveal`（滑块 surface reveal，见第 9.4 节） |

入站连接弹层可在 header 使用 `border-primary/25 bg-primary/5` 表示「需用户决策」。

空状态：`rounded-xl border border-dashed border-border py-10 text-center text-sm text-muted-foreground`。

---

## 8. 布局与浮层 z-index

### 8.1 主布局

```text
┌─────────────────────────────────────────────────────────┐
│  Sidebar (w-60 / xl:w-64)  │  Main + 可选 DevicesPanel │
│  剪贴板 / 设备 / 设置       │  按 activeNav 切换        │
└─────────────────────────────────────────────────────────┘
```

根容器：`flex h-screen overflow-hidden bg-background text-foreground`。

### 8.2 浮层栈（禁止随意新造 z 值）

| z-index | 组件 |
|---------|------|
| 10–40 | 页内 sticky、tooltip |
| **50** | `PairingModal` |
| **60** | `IncomingConnectionPrompt` |
| **70** | `TransferProgressCard`、`ConnectionAttemptCard`（右下角） |
| **80** | `StatusNotice`（底部居中 Toast，最上） |

多块右下角卡片同时存在时，在 `App` 层 **纵向堆叠 `gap-3`**，避免重叠。

### 8.3 模态无障碍

- `role="dialog"`、`aria-labelledby` / `aria-describedby`（入站请求已示范；配对模态应对齐）
- 全局提示：`aria-live="polite"`（`StatusNotice`）
- 键盘：`focus-visible:ring-2 ring-ring ring-offset-2 ring-offset-background`（按钮与导航推荐）

---

## 9. 动画与转场

**技术**：Tailwind `transition-*`、`tw-animate-css`、必要时 `styles` 内自定义 `@keyframes`（如连接尝试 marquee）。

**原则**：短、一致、可关闭。常规微交互（焦点环、开关滑块位移、主题色点缩放等）多数 ≤ 250ms；**表面背景淡入淡出**单独使用 Surface Reveal 令牌（略长，见 9.4），不与模态/切页混用同一时长。

### 9.1 动效令牌（`index.css`）

**通用微交互**

```css
--motion-fast: 120ms;
--motion-normal: 180ms;
--motion-slow: 240ms;
--motion-enter: 220ms;
--motion-exit: 160ms;
--ease-out: cubic-bezier(0.16, 1, 0.3, 1);
--ease-in: cubic-bezier(0.4, 0, 1, 1);
```

**Surface Reveal（表面背景 / 滚动条滑块）**

```css
--surface-reveal-fade-in: 380ms;
--surface-reveal-fade-out: 520ms;
--surface-reveal-ease-in: cubic-bezier(0.4, 0, 0.2, 1);
--surface-reveal-ease-out: cubic-bezier(0.4, 0, 0.2, 1);
--button-primary-hover-bg: color-mix(in srgb, var(--primary) …); /* 浅/深主题各一套 */
--scrollbar-fade-in: var(--surface-reveal-fade-in);
--scrollbar-fade-out: var(--surface-reveal-fade-out);
--scrollbar-ease-in: var(--surface-reveal-ease-in);
--scrollbar-ease-out: var(--surface-reveal-ease-out);
```

### 9.2 分类（模态 / 切页 / 提示）

| 场景 | 做法 |
|------|------|
| 模态进入 | 遮罩 fade；面板 opacity + `scale(0.96→1)` + 可选 `translateY(8px→0)`，~220ms（`--motion-enter`） |
| 模态退出 | ~160ms（`--motion-exit`），卸载前播完 |
| 侧栏切页 | fade + 轻微水平位移（~6px），勿全屏 slide |
| StatusNotice | slide-up + fade；关闭前 fade 再卸载 |
| 加载 | `Loader2` + `animate-spin` |
| 配对紧迫 | 仅 `isUrgent` 时使用少量 `animate-pulse` |
| 连接尝试条 | `connection-attempt-marquee`；减少动效时放慢或静态 |

模态与页面级转场定义在 `styles/motion.css`；**不要**用 Surface Reveal 时长替代上述 enter/exit。

### 9.3 `prefers-reduced-motion`

在 `surface-reveal.css`、`scrollbar.css`、`motion.css` 中合并：关闭或缩短非必要 `animation` / `transition`；marquee 可改为静态指示。

### 9.4 Surface Reveal（表面淡入淡出）

**语义**：指针移入、聚焦或滚动时，**背景色（及配套文字色 / 边框色）** 从静止态过渡到强调态；移出时 **移出略慢于移入**，避免「一闪而过 / 迟迟不退」的割裂感。

| 方向 | 时长 | 缓动 |
|------|------|------|
| 移入（hover / focus-visible / engaged） | `380ms` | `cubic-bezier(0.4, 0, 0.2, 1)` |
| 移出 | `520ms` | 同上 |

**样式与代码**

| 资源 | 说明 |
|------|------|
| `styles/surface-reveal.css` | keyframes + **`app-surface-reveal-bg`**（移出 fade-out；`:hover` / `:focus` / `:focus-visible` 移入 fade-in） |
| `styles/scrollbar.css` | WebKit 滑块复用上述 keyframes；Firefox 用 `scrollbar-color` + 同一令牌 |
| `app/constants/surfaceReveal.ts` | `SURFACE_REVEAL_BG`、`SURFACE_REVEAL_TEXT_FIELD`、`SURFACE_REVEAL_NAV_ITEM`、`SETTINGS_CONTROL_COLUMN` 等 |
| `app/hooks/useScrollbarReveal.ts` | 滚动条 `app-scrollbar-engaged` / `app-scrollbar-leaving`；**`FADE_OUT_MS` 须与 `--surface-reveal-fade-out` 一致（520）** |

**已统一应用**

- 自定义滚动条（`ScrollArea` + `app-scrollbar`）：静止时滑块透明。
- **按钮**（含主/图标/分段/设置页背景模式等）：`app-surface-reveal-bg` + 声明 hover 背景。
- **侧栏导航**：`SURFACE_REVEAL_NAV_ITEM`（15px 图标列；未选中 `bg-transparent` 与选中同 padding）。
- **输入框 / `select`**：`SURFACE_REVEAL_TEXT_FIELD`、`SURFACE_REVEAL_SELECT` 等（hover/focus 背景与边框，勿用 `transition-colors` 替代）。
- **开关**：`SettingToggle` 轨道 + `SettingToggleControl`（`w-12` 右对齐）；滑块 `left` 过渡用 `--surface-reveal-fade-in`。
- **剪贴板列表内容块**：`CLIP_LIST_PREVIEW_SURFACE`（与图片预览 hover 一致）。

**不适用 Surface Reveal（结构性转场）**

- 模态遮罩与面板、切页 `page-enter`、Toast 进出场。
- **ThemeSwatch 色点** `group-hover:scale-105`：缩放点缀，时长不必与 surface 相同。

**待协商**

| 区域 | 建议 |
|------|------|
| 设备卡片仅 `hover:border-primary/30` | 可选加浅底 + surface reveal |

### 9.5 不建议

- 默认引入 framer-motion（确有列表 stagger 等需求再评估）
- 全页 bounce、blur 入场
- 在 **非 Surface Reveal** 场景滥用 >300ms 的 hover（模态、切页除外）
- 剪贴板内容区 heavy glass

---

## 10. Figma 与设计对照

（自 [`AGENTS.md`](../AGENTS.md) 提取。）

- 可使用 `figma-mcp-go` 读取 Figma；优先 `get_design_context`，大文档避免整树拉取。
- 改 Figma 前确认节点与意图；删除、覆盖、批量重命名等须谨慎。
- MCP 权限配置在 IDE 的 MCP 设置中，不依赖本文档路径。
- **关系**：Figma Design → 视觉与结构对照；`Apps` → 运行宿主与真实能力；日常改 UI 以本文 + `index.css` 为准。

历史主题：[`Plans/2026-06-21-[LOCKED]-ui-refresh-figma-integration`](../Plans/2026-06-21-[LOCKED]-ui-refresh-figma-integration/)。

---

## 11. 样式工程

| 路径 | 说明 |
|------|------|
| `Apps/src/main.tsx` | 仅 import `./styles/index.css` |
| `Apps/src/styles/index.css` | **唯一**全局样式入口：字体、`:root` / `.dark` 令牌、`@theme inline`；`@import` `motion.css`、`surface-reveal.css`、`scrollbar.css` |
| `Apps/src/styles/surface-reveal.css` | Surface Reveal keyframes + `app-surface-reveal-bg`（勿放入 `@layer`，WebView2 需 unlayered） |
| `Apps/src/styles/scrollbar.css` | 滚动条伪元素与 `app-scrollbar` 状态类 |
| `Apps/src/styles/motion.css` | 模态、切页、Notice 等结构性动画 |
| `Apps/src/app/constants/surfaceReveal.ts` | `SURFACE_REVEAL_BG` 类名常量 |
| `Apps/src/app/constants/theme.ts` | 主题色 id 与默认 UI 设置 |

勿在 `theme.css` 等未引用文件中维护第二份令牌；冗余文件应合并进 `index.css` 或删除。

---

## 12. 跨设备隐私（与 UI 展示相关）

（自 [`AGENTS.md`](../AGENTS.md) 摘录，影响 UI 文案与状态展示。）

- 不得在他机 UI 中暴露本机配置名、内部实现；对外原因码映射为用户可见中文。
- 发送端不得根据他机行为推断其配置并展示；`peer:handled` 等等效成功须视为成功反馈。

完整条文见 [`AGENTS.md`](../AGENTS.md)「跨设备隐私与协议」。

---

## 13. 实施与变更

- 分阶段落地 checklist：[`Plans/2026-06-30-ui-design-system/execution-plan.md`](../Plans/2026-06-30-ui-design-system/execution-plan.md)
- 讨论记录与细目：同目录下 `01-foundation-tokens.md`、`02-components-and-polish.md`、`03-motion-and-transitions.md`
- 更新本规范时：若与 `AGENTS.md` 文案/交互条款冲突，**以本文件与 `AGENTS.md` 同步修改为准**（`AGENTS.md` 保留摘要与链接）
