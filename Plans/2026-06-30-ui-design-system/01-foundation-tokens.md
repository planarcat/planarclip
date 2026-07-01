# UI 规范 · 基础令牌与排版

> 主题：`2026-06-30-ui-design-system`  
> 依据：`Apps/planarclip/src/styles/index.css`、`constants/theme.ts`、现有组件 class 用法  
> 与 Figma：延续 `2026-06-21-ui-refresh-figma-integration` 结论——Design 为视觉真源，实现以 **CSS 变量 + Tailwind** 为准，不引入第二套色板。

## 1. 设计原则（产品向）

| 原则 | 含义 |
|------|------|
| 工具感优先 | 信息密度适中、操作就近；避免大面积装饰与长动画 |
| 令牌单一来源 | 颜色/圆角/阴影/动效时长只从 CSS 变量或约定 class 读取 |
| 状态可感知 | 连接、配对、传输、错误四类流程必须有统一视觉语言 |
| 中文为主 | 字号与行高保证 13px 辅助文案在 Win 上清晰可读 |
| 尊重系统 | `prefers-reduced-motion` 下弱化或关闭非必要动效 |

## 2. 色彩语义（沿用并收紧）

### 2.1 表面层级（暗色为默认体验）

```text
background (#090c14)     → 应用画布
card (#0f1220)           → 侧栏、主卡片、弹层面板
secondary (#161b2e)      → 嵌套块、输入底、列表项底
muted / muted-foreground → 分隔区、说明文案
primary + ring           → 主操作、焦点环、品牌强调（随 theme_color 切换）
destructive              → 断开、拒绝、超时紧迫态
border                   → 统一描边（暗：rgba(255,255,255,0.07)）
```

### 2.2 主题色四套（`THEME_COLORS`）

实现层已通过 `useUiTheme` 写入 `--primary` / `--ring` / `--accent` 等，**禁止**在组件内硬编码 `#22d3ee` 等 hex（配对进度条等已有 `bg-primary` 的保持）。

### 2.3 语义色扩展（建议新增 CSS 变量，可选）

在 `index.css` `:root` / `.dark` 中补充，用于成功/警告而不滥用 `primary`：

| 令牌 | 用途 | 建议值（暗色） |
|------|------|----------------|
| `--success` | 配对成功、同步完成 | 沿用 `--chart-2` `#34d399` |
| `--warning` | 即将超时、需注意 | 沿用 `--chart-4` `#f59e0b` |
| `--overlay` | 模态遮罩 | `rgb(0 0 0 / 0.6)`（与现 `bg-black/60` 一致） |

Tailwind 映射：在 `@theme inline` 增加 `--color-success` 等，组件用 `text-success` / `border-success/30`。

## 3. 圆角（统一命名）

当前混用 `rounded-md` / `lg` / `xl` / `2xl` / 任意 `rounded-[5.5px]`。规范如下：

| 令牌 | Tailwind | 像素（--radius=8px） | 用途 |
|------|----------|----------------------|------|
| radius-sm | `rounded-sm` | 4px | 极少用 |
| radius-md | `rounded-md` | 6px | 图标按钮、小输入、侧栏 logo 块 |
| radius-lg | `rounded-lg` | 8px | 默认按钮、输入框、nav 项 |
| radius-xl | `rounded-xl` | 12px | **标准卡片**、设备行内块、列表容器 |
| radius-2xl | `rounded-2xl` | 16px | **模态面板**、底部 Toast、传输卡 |
| full | `rounded-full` | — | 开关、状态点、进度条轨道 |

**待收敛**：`DevicesPage` 中 `rounded-[5.5px]` 改为 `rounded-md`。

## 4. 间距与布局

### 4.1 栅格

- 侧栏固定：`w-52` / `xl:w-56`（保持）
- 主内容：`min-w-0 flex-1` + 子页内部 `px-6 py-5` 或 `p-5`（剪贴板 / 设备 / 设置应对齐同一水平 padding）
- 弹层：`max-w-[360px]`（配对）/ `max-w-sm`（入站请求）——统一为 **`max-w-[380px]`** 模态宽度档，小屏 `mx-4`

### 4.2 间距刻度（仅使用下列值）

`1` `1.5` `2` `2.5` `3` `4` `5` `6` — 对应 Tailwind spacing。区块纵距默认 `space-y-4` 或 `gap-4`；卡片内 `p-3` 或 `p-4`；弹层节与节 `space-y-5`（配对模态已用）。

### 4.3 边框与 hover

- 默认卡片：`border border-border bg-card`
- 可点击卡片 hover：`transition-colors hover:border-primary/30`（已广泛使用，定为标准）
- 主按钮 hover：`hover:opacity-90` 或 `hover:brightness-110`（二选一，全 app 统一为 **opacity**）
- 破坏性图标底：`bg-destructive/12 hover:bg-destructive/20`（`DisconnectIconButton` 为准）

## 5. 阴影与景深

| 层级 | class | 场景 |
|------|-------|------|
| 无 / 微 | 无或 `shadow-sm` | 侧栏 nav 选中块 |
| 卡片浮起 | `shadow-md` | 可选：右侧 DevicesPanel 与主区分 |
| 模态 / Toast | `shadow-2xl` | PairingModal、StatusNotice、IncomingConnectionPrompt |
| 遮罩 | `backdrop-blur-sm` + `--overlay` | 全屏模态（已用） |

不建议全局加 heavy blur；桌面工具保持利落。

## 6. 排版

### 6.1 字体

- 界面：`Plus Jakarta Sans` + 中文回退（`index.css` body 已配置）
- 等宽：配对码、地址 → `font-mono`（JetBrains Mono 已加载）
- **不要**在正文中混用第三种字体

### 6.2 字号阶梯（收敛 arbitrary）

| 角色 | class | 约 px | 用途 |
|------|-------|-------|------|
| 页面标题 | `text-base font-semibold` | 15 | 各页顶栏标题 |
| 正文 | `text-sm` | 14 | 列表主文案、按钮 |
| 辅助 | `text-[13px]` → 逐步改为 `text-sm` 或定义 `text-ui-secondary` | 13 | 说明、入站描述 |
| 标签 / 元数据 | `text-xs font-medium` | 12 | 时间、状态、section 标签 |
| 微字 | `text-[11px]` → 统一为 `text-xs` | 12 | 副标题、配对模态说明 |

**目标**：减少 `text-[10px]`/`text-[11px]` 碎片，最多保留 **xs + sm + base** 三档 + mono 配对码 `text-base tracking-[0.2em]`。

### 6.3 字重

- 标题 / 按钮：`font-semibold` 或 `font-medium`（导航项 `font-medium`）
- 正文：`font-normal` / `font-medium`（列表项标题用 medium）

## 7. 图标

- 库：**lucide-react**（保持）
- 尺寸：侧栏/导航 `15`；行内操作 `14`；模态头 `15–18`；空状态可 `20`
- 纯图标按钮：必须 `aria-label` 或 `title`（AGENTS.md 约定）

## 8. Z-index 栈（现网整理 + 规范）

| 值 | 层 |
|----|-----|
| z-10 ~ z-40 | 页内 sticky、tooltip（`DevicesPage` tooltip z-20） |
| **z-50** | 配对模态 `PairingModal` |
| **z-[60]** | 入站连接 `IncomingConnectionPrompt` |
| **z-[70]** | 右下角 `TransferProgressCard` / `ConnectionAttemptCard` |
| **z-[80]** | 全局 `StatusNotice`（最上，避免被挡） |

新增浮层必须从此表取值，禁止随意 `z-[90]`。

## 9. 无障碍与焦点

- 模态：`role="dialog"` + `aria-labelledby`（入站已有，配对模态应补齐）
- 焦点：输入框 `focus:border-primary focus:outline-none`；后续可为模态内焦点陷阱（可选阶段）
-  live 区域：`StatusNotice` 已 `aria-live="polite"`

## 10. 工程备注

- 样式入口仅为 `styles/index.css`；`styles/theme.css` 与 `tailwind.css` 当前**未引用**，实施时合并进 `index.css` 或删除冗余，避免双份 token。
