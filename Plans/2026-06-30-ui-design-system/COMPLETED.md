# UI 规范落地 — 已执行确认

> 主题：`2026-06-30-ui-design-system`  
> 执行时间：2026-06-30  
> 规范正文：[`docs/UI_GUIDE.md`](../../docs/UI_GUIDE.md)

## 已完成

### 阶段 0

- `index.css` 增加 `--motion-*`、`--ease-*`、`--success` / `--warning` 与 Tailwind 映射
- 新增 `styles/motion.css` 并由 `index.css` 引入
- 删除未引用的 `styles/theme.css`、`styles/tailwind.css`
- `pnpm check:web` 通过

### 阶段 1

- `DevicesPage`、`PairingModal`、`DisconnectIconButton` 收敛圆角与 `text-xs`
- 模态宽度统一为 `max-w-[380px]`（`ModalShell`）
- `PairingModal`：`role="dialog"`、`aria-modal`、`aria-labelledby` / `aria-describedby`

### 阶段 2

- 新增 `app/components/ui/`：`ModalShell`、`PrimaryButton`、`IconButton`、`PageHeader`、`EmptyState`
- `IncomingConnectionPrompt`、`PairingModal` 迁移至 `ModalShell`

### 阶段 3

- `hooks/useOverlayLifecycle.ts`
- 模态进入/退出动画（`ModalShell`）
- `StatusNotice` 进入/退出动画
- 侧栏切页 `page-enter`（`App.tsx` `key={activeNav}`）
- `prefers-reduced-motion` 合并于 `motion.css`

### 阶段 4

- 右下角 `ConnectionAttemptCard` + `TransferProgressCard` 堆叠（`anchored={false}` + App 容器）
- 暗色/亮色 body 微渐变背景

## 未执行（按方案可选 / 后续）

- 阶段 5：Figma 对照微调
- 全站 `PageHeader` 组件替换各页顶栏（仅提供组件，未批量替换 `ClipboardPage` / `DevicesPage` / `SettingsPage`）
- `SurfaceCard` / `TextField` 独立组件文件（契约已落在 `UI_GUIDE`，可按需再抽）
- 侧栏 `Sidebar` 等文件的 `text-[10px]`/`text-[11px]` 存量收敛

## 验证建议

- `pnpm dev:web`：切页动效、主题切换
- `pnpm dev`：配对弹层开闭、入站请求、传输卡与 Notice 叠放
