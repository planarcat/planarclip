# 前端覆盖率基线

> 生成时间: 2026-07-11
> 触发命令: `pnpm test:web:coverage`
> Vitest: 2.1.9 · provider: v8
> 用例总数: 67 tests / 6 files（全部通过）

## 全量汇总

| 维度 | Statements | Branches | Functions | Lines |
|:---|---:|---:|---:|---:|
| **All files** | **7.46%** | **72.22%** | **41.23%** | **7.46%** |

说明：Statements/Lines 值低是**预期结果**——Phase 2 仅覆盖 `utils/` + `hooks/` 两个纯逻辑层；`components/`、`useConnectionBridge`、`usePairingFlow`、`useUiTheme` 等大文件目前 0 覆盖，稀释了整体值。Branches 72% 说明**已覆盖模块中的分支密度较高**。

## 已完全覆盖（100%）

| 文件 | Lines | Branches | Functions |
|:---|---:|---:|---:|
| `app/hooks/useOverlayLifecycle.ts` | 100 | 100 | 100 |
| `app/hooks/usePairingCountdown.ts` | 100 | 100 | 100 |
| `app/utils/time.ts` | 100 | 100 | 100 |
| `app/constants/theme.ts` | 100 | 100 | 100 |

## 部分覆盖

| 文件 | Lines | 未覆盖分支 |
|:---|---:|:---|
| `app/utils/device.ts` | 99.38% | 155（trusted 兜底名称字段的次级分支） |
| `app/utils/message.ts` | 82.87% | 138-239 区间的少量兜底分支（浏览器预览提示、其余递归剥离） |
| `app/utils/settings.ts` | 67.07% | `applyThemeColor` / `applyColorScheme` / `applyAppearanceFromUiSettings`（DOM 副作用未测） |

## 未覆盖（0%）— 属于本 Phase 计划外

- `app/components/**`（组件测试延后到 Phase 2 之外）
- `app/hooks/{useConnectionBridge, usePairingFlow, useUiTheme, useSyncSettings, ...}`（Tauri 桥接类 hook，需要更细的 Tauri mock 场景，延后）
- `app/utils/{clipboard, transfer, themeTokenBlend, themeTransition, appearanceBootstrap, scheduleDeferred}`（UI/剪贴板副作用与主题混色，价值低或需集成测试）
- `app/constants/{app, clipboard, clipPreviewSurface, connection, surfaceReveal, sync}`（纯常量导出，无逻辑）

## 使用建议

- **不设阈值**：现值 7.46% 若立刻作为门禁会**阻塞后续任何删除文件的清理**，与仓库现状不符
- **观察窗口**：2~3 周后重新采集，若 lines 稳定在 X%，可设 `thresholds.lines: max(X - 5, 5)` 作为回退保护
- **报告位置**：`Apps/coverage/index.html`（本地打开）；CI 接入时可用 `json-summary` 作为门禁比较基准

## 复现

```
pnpm test:web:coverage
```

产物：`Apps/coverage/`（已在根 `.gitignore` 中）