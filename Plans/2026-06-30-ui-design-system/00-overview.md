# PlanarClip UI 规范总览

基于当前 `Apps/planarclip` 实现整理的一套**可执行** UI / 美化 / 动效方案，不推翻现有 Tailwind + CSS 变量架构。

**项目级正文（权威）**：[`docs/UI_GUIDE.md`](../../docs/UI_GUIDE.md) — 含自 `AGENTS.md` 提取的文案/交互/Figma 约定；本目录为讨论细目与落地 checklist。

## 文档索引

| 文件 | 内容 |
|------|------|
| [01-foundation-tokens.md](./01-foundation-tokens.md) | 色彩层级、圆角、间距、排版、阴影、z-index |
| [02-components-and-polish.md](./02-components-and-polish.md) | 组件 class 契约、页面美化、现状差异 |
| [03-motion-and-transitions.md](./03-motion-and-transitions.md) | 动效令牌、模态/切页/业务动画、无障碍 |
| [execution-plan.md](./execution-plan.md) | 分阶段落地清单与测试 |

## 架构快照

```text
┌─────────────────────────────────────────────────────────┐
│  Sidebar (card)  │  Main: Page + optional DevicesPanel │
│  nav / 本机名    │  clipboard | devices | settings      │
└─────────────────────────────────────────────────────────┘
        ↑ overlays: z-50 配对 → z-60 入站 → z-70 右下卡 → z-80 Toast
```

## 与历史方案关系

- 视觉真源仍可对齐 Figma Design（见已锁定 `2026-06-21-ui-refresh-figma-integration`）
- 本主题侧重：**在已上线 UI 上收敛令牌 + 补动效 + 薄组件层**，避免再次全量 Make 迁移
