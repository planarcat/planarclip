# 未执行 / 遗留事项

> 生成时间: 2026-07-11
> 来源主题: 2026-07-11-测试基建搭建
> 说明: 本文档记录当前主题执行后仍未完成、未处理或计划未来处理的事项

---

## ⏳ 仍待处理的遗留事项

### ❌ 执行失败

（无）

### ⚠️ 未开发完成 / 完成度不足

| 事项 | 当前状态 | 差距描述 | 后续建议 |
|:---|:---|:---|:---|
| `clipboard_sync_e2e` 同进程双引擎集成测试 | 未做 | 需要引入 `ClipboardSource` trait 并解耦 `SyncEngine` 对 `AppHandle` 的直接依赖 | 单独开一个"SyncEngine 可测性抽象"主题，先做 `gitnexus_impact` 分析，再讨论是否值得抽象 |
| `Apps/src-tauri/tests/` 外置集成测试目录 | 未启用 | 生产模块目前均为 `mod xxx`（crate 私有），外置测试无法访问 | 若未来需要，评估将 `network` / `network::direct` / `network::protocol` 提升为 `pub mod`，或使用 `#[cfg(test)] pub use` 二次导出 |

### 📅 被移出本次开发节奏

| 事项 | 移出原因 | 优先级 | 参考讨论 |
|:---|:---|:---|:---|
| `Docs/TESTING.md` 详细手册 | AGENTS.md 里的「测试」小节已覆盖脚本 / 位置 / mock 用法，暂无更细需求 | 低 | [01-测试基建整体方案.md](01-测试基建整体方案.md) |
| 组件层测试（`Apps/src/app/components/**`） | Tauri mock 场景待稳定；`PairingModal` 等组件依赖多个 hook，先让 hook 层稳固再上组件测试 | 中 | [execution-plan.md](execution-plan.md) Phase 2 |
| `insta` 快照测试 | 本轮 Rust 单测均用直接断言即可；协议帧序列化未复杂到需要快照 | 低 | 保留 `[dev-dependencies]` 位置留白可后加 |

### 🔮 计划在未来版本加入

| 事项 | 计划版本/Phase | 参考讨论 |
|:---|:---|:---|
| SyncEngine 可测性抽象（`ClipboardSource` trait + 解耦 `AppHandle`），完成后补 `clipboard_sync_e2e` | 长期 | [execution-plan.md](execution-plan.md) Phase 4 |
| 组件层测试（`PairingModal` / `IncomingConnectionPrompt` / `StatusNotice`） | 中期 | Phase 2 收尾 |
| Playwright 浏览器模式 UI 冒烟 E2E | 中期（前 3 层稳定后） | [02-决策确认与E2E取舍.md](02-决策确认与E2E取舍.md) |
| `tauri-driver` + WebdriverIO 真实桌面 E2E | 长期（有发布节奏后评估） | [02-决策确认与E2E取舍.md](02-决策确认与E2E取舍.md) |
| GitHub Actions CI，跑 `pnpm check && pnpm test` | 未定 | [02-决策确认与E2E取舍.md](02-决策确认与E2E取舍.md) |
| 2~3 周后依据覆盖率基线设定 threshold（回退保护而非硬门禁） | 未定 | [02-决策确认与E2E取舍.md](02-决策确认与E2E取舍.md) / [coverage-baseline.md](coverage-baseline.md) |
| `useConnectionBridge` / `usePairingFlow` / `useUiTheme` 等 Tauri 桥接类 hook 的单测 | 中期 | 需要更细的 Tauri mock 场景铺垫 |

---

## 🗑️ 已放弃事项（仅记录）

（无）