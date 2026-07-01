# Plans 目录说明

本目录遵循 **plan-discussion / plan-execution / plan-lock** 三技能约定。方案讨论只改此处文档，不改源码。

## 目录结构

| 路径 | 含义 |
|:---|:---|
| `Plans/{YYYY-MM-DD}-{主题}/` | **进行中**主题（可讨论、可执行、可改方案） |
| `Plans/归档/{YYYY-MM-DD}-{主题}/` | **已锁定**主题（只读；锁定 = 位于此目录） |
| `Plans/未完成池/*.md` | 已归档主题的未完成事项**精简摘要**（跨主题查阅） |
| `Plans/_backlog/` | **遗留**，已迁移至 `未完成池/`；扫描时与 `未完成池/` 一并排除 |

## 主题内固定文件

- `execution-plan.md` — 待执行方案
- `COMPLETED.md` — 执行结果确认（执行完成后草稿）
- `UNEXECUTED.md` — 未执行 / 遗留事项（执行完成后草稿）
- `01-…md`、`02-…md` — 各轮讨论
- `附件/` — 讨论图片与素材（旧主题可能仍为 `assets/`，只读兼容）

**不再使用：** 文件夹名 `[LOCKED]` 前缀、`STATUS.md`。旧主题已按新结构迁移（2026-07-01）。

## 当前状态（2026-07-01）

### 进行中（`Plans/` 根下）

| 主题 | 说明 |
|:---|:---|
| `2026-05-26-cross-platform-clipboard-sync` | 早期架构讨论；有 `COMPLETED.md`，未走新流程锁定 |
| `2026-06-05-lan-mdns-auto-discovery` | 局域网发现；已执行，未锁定 |
| `2026-06-24-pairing-modal-refactor` | 配对弹层；已执行，未锁定 |
| `2026-06-26-connection-flow-ux-polish` | 连接流程 UX；仅 `COMPLETED.md` |
| `2026-06-26-destroy-webview-on-close` | 关窗销毁 WebView；有执行总结，未锁定 |
| `2026-06-30-planarclip-presence-probe` | 在线/离线探测与 flicker；**最新执行完成草稿** |
| `2026-06-30-ui-design-system` | UI 设计系统；**最新执行完成草稿** |

需要归档时，请明确说「锁定主题」以触发 **plan-lock**（须具备三清单闭环）。

### 已归档（`Plans/归档/`）

| 主题 | 未完成摘要 |
|:---|:---|
| `2026-06-04-single-machine-connection-testing` | [未完成池/2026-06-04-single-machine-connection-testing.md](未完成池/2026-06-04-single-machine-connection-testing.md) |
| `2026-06-18-vite-plus-adoption` | [未完成池/2026-06-18-vite-plus-adoption.md](未完成池/2026-06-18-vite-plus-adoption.md) |
| `2026-06-21-ui-refresh-figma-integration` | [未完成池/2026-06-21-ui-refresh-figma-integration.md](未完成池/2026-06-21-ui-refresh-figma-integration.md) |
| `2026-06-22-app-tsx-splitting-refactor-verified` | [未完成池/2026-06-22-app-tsx-splitting-refactor-verified.md](未完成池/2026-06-22-app-tsx-splitting-refactor-verified.md) |
| `2026-06-26-multi-connection-sessions` | [未完成池/2026-06-26-multi-connection-sessions.md](未完成池/2026-06-26-multi-connection-sessions.md) |
| `2026-06-27-image-file-sync` | [未完成池/2026-06-27-image-file-sync.md](未完成池/2026-06-27-image-file-sync.md) |

## 新建主题命名（摘要）

- 文件夹：`YYYY-MM-DD-{中文主题简称}`（新建默认中文；历史英文名保留）
- 讨论文件：`01-{中文问题摘要}.md`
- 详见全局 skill：`plan-discussion` §命名与语言规范
