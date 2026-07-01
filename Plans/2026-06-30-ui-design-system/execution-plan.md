# 待执行方案 · UI 规范落地

> 主题：`2026-06-30-ui-design-system`  
> **项目级规范正文**：[`docs/UI_GUIDE.md`](../../docs/UI_GUIDE.md)  
> **已执行摘要**：[`COMPLETED.md`](./COMPLETED.md)  
> 范围：仅前端 `Apps/planarclip/src`；不改变 Rust 协议。

## 阶段 0：基线（0.5 天）

- [x] 在 `index.css` 增加 `--motion-*`、`--ease-*`（及可选 `--success`）
- [x] 合并或删除未使用的 `styles/theme.css`、`styles/tailwind.css`
- [x] `pnpm check:web` 通过

## 阶段 1：令牌收敛（1 天）

- [x] 统一圆角 / 字号：替换 `rounded-[5.5px]`、`text-[10px]`/`text-[11px]` 为高优先级页面（DevicesPage、PairingModal）
- [x] 统一主按钮 hover 为 `opacity`
- [x] 模态宽度 `max-w-[380px]`、z-index 按 01 文档
- [x] PairingModal 补 `role="dialog"` + `aria-*`

## 阶段 2：公共组件薄层（1–2 天）

- [x] 新增 `components/ui/`：`EmptyState`、`PrimaryButton`、`IconButton`、`ModalShell`（`PageHeader` 已加）
- [x] 迁移 1 个 overlay（`IncomingConnectionPrompt`）验证 API
- [x] 迁移 `PairingModal` 外壳与按钮

## 阶段 3：动效（1 天）

- [x] `styles/motion.css` + `useOverlayLifecycle`
- [x] 模态进入/退出
- [x] `StatusNotice` 进入/退出
- [x] 侧栏 `activeNav` 页面 `page-enter`
- [x] 合并 `prefers-reduced-motion` 规则

## 阶段 4：美化与编排（1 天）

- [ ] 各页 `PageHeader` 间距对齐（组件已就绪，页面未全量替换）
- [ ] 空状态样式统一（`EmptyState` 已用于配对列表；剪贴板页待对齐）
- [x] App 层：TransferProgress + ConnectionAttempt 右下角堆叠 `flex flex-col gap-3`
- [x] 可选：暗色背景微渐变

## 阶段 5：对照 Figma（按需）

- [ ] 用 Figma MCP 读设置页节点，微调 padding/分组标题
- [ ] 更新 `Resources/PlanarClip` 或 Design 注释（非阻塞）

## 测试计划

- `pnpm dev:web`：三页切换、主题色/明暗、设置开关  
- `pnpm dev`：配对开闭、入站请求、传输进度与 Notice 同时出现  
- Windows：125% 缩放下的 12px 辅助字可读性  
- 开启系统「减少动态效果」验证 marquee 与模态

## 不在本主题内

- 剪贴板业务逻辑、多连接会话（另主题）
- 引入 framer-motion / shadcn 全量
- 重新生成 Figma Make 全量 UI
