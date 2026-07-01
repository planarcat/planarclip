# Vite+ 接入评估与迁移方案 — 待执行方案

> 生成时间: 2026-06-18
> 基于讨论: [01-what-is-vite-plus.md](01-what-is-vite-plus.md) | [02-is-vite-plus-a-good-fit.md](02-is-vite-plus-a-good-fit.md) | [03-how-to-migrate-to-vite-plus.md](03-how-to-migrate-to-vite-plus.md)
> 外部参考: [90-vite-plus-official-sources.md](90-vite-plus-official-sources.md)

## 需求概述

围绕 PlanarClip 当前仓库，评估 Vite+ 的定位、判断当前阶段是否值得迁移，并在“未来决定接入”的前提下，形成一套适合现状的最小可用迁移方案。目标不是立即执行全面重构，而是明确：

- Vite+ 到底解决什么问题；
- 当前项目是否已经到了值得迁移的阶段；
- 如果未来要上，应该按什么顺序改造，才能在控制风险的同时拿到统一工具链收益。

## 技术决策

| 决策项 | 选择 | 理由 | 来源轮次 |
|--------|------|------|----------|
| Vite+ 的定位 | 统一 Web 工具链入口 | 价值在于统一 dev/check/test/build/run，而非单独替换 Vite | 第 1 轮 |
| 当前是否立刻迁移 | 暂不建议作为高优先级事项 | 当前前端过轻，主复杂度在 Rust/Tauri | 第 2 轮 |
| 未来接入方式 | 最小可用迁移 | 当前项目规模更适合小步接入而非全面重构 | 第 3 轮 |
| 包管理器策略 | 先统一后迁移 | 当前 npm / pnpm 痕迹混杂，Vite+ 会受其影响 | 第 2、3 轮 |
| Vite 版本基线 | 先升级到 Vite 8+ | 官方迁移建议如此，可降低兼容风险 | 第 3 轮 |
| 迁移后最小收益项 | 至少落地 `vp check` | 否则只是换命令名，没有真正获得统一工具链收益 | 第 3 轮 |
| Tauri 联动策略 | 同步修改 `beforeDevCommand` / `beforeBuildCommand` | Tauri 是当前前端命令入口锚点 | 第 2、3 轮 |
| 文档处理策略 | README / CLAUDE 同步更新 | 避免命令入口与文档描述脱节 | 第 3 轮 |
| 风险控制策略 | 小步迁移 + 充分验证 | Vite+ 仍处于 Alpha，应避免一次性大改 | 第 1、3 轮 |

## 架构设计

### 当前状态

```text
Tauri CLI
  ├─ beforeDevCommand  -> npm run dev -> vite
  ├─ beforeBuildCommand -> npm run build -> tsc && vite build
  └─ Rust / Cargo / Bundle pipeline
```

特点：

- 前端层很薄，主要承担 UI 与 Tauri 事件交互。
- Rust / Tauri 才是当前项目的主要复杂度承载区。
- 当前没有完整的 test / lint / format 体系。

### 目标状态（最小可用迁移）

```text
Tauri CLI
  ├─ beforeDevCommand  -> vp dev
  ├─ beforeBuildCommand -> vp build
  └─ Rust / Cargo / Bundle pipeline

Vite+ Unified Frontend Toolchain
  ├─ vp install
  ├─ vp dev
  ├─ vp check
  ├─ vp build
  └─ (future) vp test
```

特点：

- 只统一前端工具链，不替换 Cargo / Tauri 后端构建。
- 先吃到命令入口统一和 `vp check` 的收益。
- 保留后续扩展到测试、hooks、workspace 的空间。

## 范围边界

### 当前纳入范围

- Vite+ 价值判断与迁移前提梳理
- 包管理器统一策略
- `package.json` / `vite.config.ts` / `src-tauri/tauri.conf.json` 的迁移入口识别
- 文档命令面同步更新
- 最小迁移验证链路设计

### 当前不纳入范围

- 立即执行代码改造
- 一步到位补齐完整测试体系
- 让 Vite+ 统一 Rust / Cargo / Tauri 后端构建链
- 直接引入多包 workspace 结构
- 额外引入大规模 hooks / staged / monorepo 规范

## 实现步骤

### Phase 1: 迁移前收口基础状态
- [ ] 选定单一包管理器策略（npm 或 pnpm）
- [ ] 清理与选定策略冲突的锁文件与多余痕迹
- [ ] 在 `Apps/planarclip/package.json` 中显式声明 `packageManager`
- [ ] 先处理 `README.md` 的 merge conflict，避免文档基线不稳定

### Phase 2: 升级到迁移前基线
- [ ] 将 `vite` 升级到 8+
- [ ] 评估 TypeScript 与现有 Vite/Tauri 配置兼容性
- [ ] 确认当前前端可在升级后继续通过 `npm run dev/build` 正常运行

### Phase 3: 接入 Vite+
- [ ] 引入 `vite-plus`
- [ ] 执行 `vp migrate`
- [ ] 核对 `package.json` 脚本是否切换为 `vp dev / vp build / vp preview`
- [ ] 核对 `vite.config.ts` 是否保留了 Tauri 相关端口与 HMR 逻辑

### Phase 4: 联动 Tauri 与基础工程化
- [ ] 修改 `src-tauri/tauri.conf.json` 中的 `beforeDevCommand` 为 `vp dev`
- [ ] 修改 `src-tauri/tauri.conf.json` 中的 `beforeBuildCommand` 为 `vp build`
- [ ] 为项目补最小的 `vp check` 流程
- [ ] 视需要再决定是否补 `vp test`

### Phase 5: 更新文档与验证
- [ ] 更新 `README.md` 中的安装、开发、构建命令说明
- [ ] 更新 `CLAUDE.md` 中的工作流命令说明
- [ ] 验证 `vp install`
- [ ] 验证 `vp check`
- [ ] 验证 `vp build`
- [ ] 验证 `npx tauri dev`
- [ ] 验证 `npx tauri build`

## 关键依赖

### 当前代码仓现状
- Tauri v2
- Vite v6（待升级）
- TypeScript
- Rust / Cargo 构建链

### 计划接入的统一入口
- Vite+
- `vp migrate`
- `vp install`
- `vp dev`
- `vp check`
- `vp build`

### 外部参考
- Why Vite+
- Migrate to Vite+
- Installing Dependencies
- Announcing Vite+ Alpha

## 风险与注意事项

- 当前项目前端较轻，迁移收益不会像中大型 Web 工作区那样显著。
- Vite+ 仍处于 Alpha 阶段，接入时应保持小步迁移与充分验证。
- 若不先统一包管理器，`vp install` 的实际行为可能与预期不一致。
- 迁移不会消除 Rust / Cargo / Tauri 侧的复杂度，后端构建链仍需独立维护。
- 若只替换命令而不落地 `vp check`，迁移价值会被大幅削弱。

## 推荐结论

### 当前建议
- 当前阶段**不建议立刻执行迁移**，优先级低于基础工程卫生治理。

### 未来若决定迁移
- 推荐采用 **“方案 A：最小迁移 + 补 `vp check`”**。
- 即先统一命令面与包管理器策略，再决定是否继续扩展到测试、hooks 与 workspace。

## 参考讨论

- [01-what-is-vite-plus.md](01-what-is-vite-plus.md)
- [02-is-vite-plus-a-good-fit.md](02-is-vite-plus-a-good-fit.md)
- [03-how-to-migrate-to-vite-plus.md](03-how-to-migrate-to-vite-plus.md)
- [90-vite-plus-official-sources.md](90-vite-plus-official-sources.md)
