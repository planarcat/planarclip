# PlanarClip 测试基建搭建 — 待执行方案

> 生成时间: 2026-07-11
> 基于讨论: [01-测试基建整体方案.md](01-测试基建整体方案.md) | [02-决策确认与E2E取舍.md](02-决策确认与E2E取舍.md)

## 需求概述

PlanarClip 目前**没有任何自动化测试**（前端 0 用例，Rust 0 用例，无测试运行器配置）。核心业务逻辑（配对、加密、协议帧、剪贴板同步、去重）都在 Rust 侧，每次改动只能靠双机手工联调复现，回归成本高。

本方案分三步落地测试基建：**Rust 单元测试 → 前端 Vitest → Rust 集成测试**，为后续开发建立可自动回归的安全网。E2E 短期不做，CI 暂不接入，但脚本与目录约定保持可移植性。

## 问题现状

### 已确认的空白

| 缺口 | 位置 | 影响 |
|------|------|------|
| 无 Rust 单测 | `src-tauri/src/**` 无 `#[cfg(test)] mod tests` | 密钥、去重、序列化改动无回归 |
| 无 Rust 集成测试 | `src-tauri/tests/` 目录不存在 | 协议契约变更只能靠双机联调发现 |
| 无前端测试运行器 | 无 `vitest.config.*` / `jest.config.*` | utils/hooks 改动无回归 |
| 无 Tauri IPC mock | 组件测试无法脱离 Tauri 运行 | 组件层无法测 |
| 无测试脚本 | 根/子 `package.json` 均无 `test*` 脚本 | 无统一入口 |

### 不在本方案范围

- **E2E（Playwright / tauri-driver）**：短期不做，中期评估 Playwright 浏览器冒烟，长期再看 tauri-driver（决策见第 2 轮）
- **CI（GitHub Actions）**：暂不接入，但脚本命名保持标准化以便未来无痛接入
- **覆盖率门禁阈值**：开启采集不设阈值；2~3 周后依据基线再定
- **重构现有源代码以提高可测性**：仅在必要处最小 trait 抽象，不做大规模重构

## 技术决策

| 决策项 | 选择 | 理由 | 来源轮次 |
|--------|------|------|----------|
| 前端框架 | Vitest | 与 Vite 8 天然契合，共享 `vite.config.ts` | 第 1、2 轮 |
| 前端 DOM 环境 | jsdom | 组件测试轻量，无需真实浏览器 | 第 1 轮 |
| 组件测试库 | @testing-library/react | 事实标准，语义化查询 | 第 1 轮 |
| Tauri API mock | 手写 `src/test/tauri-mock.ts` | invoke/listen/emit 三个入口足够，避免引额外框架 | 第 1 轮 |
| 覆盖率 | v8 provider，开启但**不设阈值** | 从 0 起步，2~3 周后依据基线再定 | 第 2 轮 |
| Rust dev-dependencies | tokio(test-util) + pretty_assertions + insta + tempfile | 覆盖异步 + 断言可读性 + 快照 + 临时目录四类需求 | 第 1 轮 |
| Rust 集成测试策略 | 同进程双引擎 + `127.0.0.1:0` 回环 | 替代双机手工联调，速度快且确定性高 | 第 1 轮 |
| 测试文件位置 | 前端就近 `__tests__/`，Rust 内联 + `tests/` | 与两个生态惯例对齐 | 第 1 轮 |
| 脚本命名 | `pnpm test` / `test:web` / `test:rust` | 延续仓库 `<动作>` / `<动作>:web` 惯例 | 第 1 轮 |
| E2E | 短期不做 | 核心价值在 Rust 侧，集成测试性价比更高 | 第 2 轮 |
| CI | 暂不做 | 脚本可移植，未来无痛接入 | 第 2 轮 |

## 架构设计

### 目录结构

```text
planarclip/
├── package.json                    # 增加 test / test:web / test:rust 转发脚本
└── Apps/
    ├── package.json                # 增加 test / test:web / test:web:watch / test:web:ui
    ├── vitest.config.ts            # 新增：jsdom + setup + coverage(v8)
    ├── .gitignore                  # 增加 coverage/
    ├── src/
    │   ├── test/                   # 新增：测试基础设施
    │   │   ├── setup.ts            # jest-dom 加载 + 全局 tauri mock 注入
    │   │   └── tauri-mock.ts       # invoke / listen / emit 可编程 stub
    │   ├── app/
    │   │   ├── utils/
    │   │   │   └── __tests__/      # 就近放测试，覆盖纯函数
    │   │   │       ├── device.test.ts
    │   │   │       ├── message.test.ts
    │   │   │       ├── time.test.ts
    │   │   │       └── settings.test.ts
    │   │   ├── hooks/
    │   │   │   └── __tests__/      # renderHook 覆盖
    │   │   │       ├── usePairingCountdown.test.ts
    │   │   │       └── useOverlayLifecycle.test.ts
    │   │   └── components/         # 组件测试延后（Step 2 收尾）
    │   └── ...
    └── src-tauri/
        ├── Cargo.toml              # 增加 [dev-dependencies]
        ├── src/
        │   ├── crypto/keys.rs      # 底部加 #[cfg(test)] mod tests
        │   ├── sync/dedup.rs       # 同上
        │   ├── network/protocol.rs # 同上（frame encode/decode + peer:* 映射）
        │   ├── storage/json.rs     # 同上（默认值/损坏文件/迁移）
        │   └── clipboard/history_preview.rs  # 同上（摘要/边界）
        └── tests/                  # 新增：跨模块集成测试
            ├── loopback_direct.rs      # TCP 回环 + 帧收发
            ├── pairing_handshake.rs    # 6 位码 + X25519 双端握手
            └── clipboard_sync_e2e.rs   # 同进程双引擎，文本/图片/文件三路
```

### 脚本入口

**根 `package.json`：**

```json
"test": "pnpm --filter planarclip test",
"test:web": "pnpm --filter planarclip test:web",
"test:web:watch": "pnpm --filter planarclip test:web:watch",
"test:rust": "cargo test --manifest-path Apps/src-tauri/Cargo.toml"
```

**`Apps/package.json`：**

```json
"test": "pnpm test:web && cargo test --manifest-path src-tauri/Cargo.toml",
"test:web": "vitest run",
"test:web:watch": "vitest",
"test:web:ui": "vitest --ui",
"test:web:coverage": "vitest run --coverage"
```

### Tauri Mock 设计

`src/test/tauri-mock.ts` 用 `vi.mock` 拦截：

- `@tauri-apps/api/core` → `invoke(cmd, args)`：内部有一张 `Map<cmd, handler>`，用例用 `mockInvoke('get_settings', () => ...)` 注册
- `@tauri-apps/api/event` → `listen(event, cb)` / `emit(event, payload)`：内部有 `Map<event, Set<callback>>`，用例可 `emitEvent('lan-devices-changed', payload)` 手动触发
- `@tauri-apps/plugin-shell`：默认 no-op stub

`setup.ts` 里 `beforeEach(resetTauriMocks)` 保证用例隔离。

### Rust 集成测试可测性抽象

`sync::engine` 与 `network::direct` 目前直接依赖真实剪贴板与真实 TCP。集成测试需要在**不改动主流程语义**的前提下最小暴露：

- `direct::listen_on` 支持传入 `SocketAddr`（`127.0.0.1:0` 让 OS 分端口），已有则复用
- `sync::engine` 提供 `ClipboardSource` trait（读/写/watch），生产用 `ArboardSource`，测试用 `InMemorySource`
- 若上述抽象不存在，Phase 3 会在 Rust 集成测试之前先做最小抽象补齐；抽象改动本身走 gitnexus 影响分析后再落

## 实现步骤

### Phase 1: 脚手架 & 前端 Vitest 骨架

- [ ] 根 `package.json` 增加 `test` / `test:web` / `test:web:watch` / `test:rust` 转发脚本
- [ ] `Apps/package.json` 增加上述 5 个脚本 + `devDependencies`（vitest / @vitest/ui / @vitest/coverage-v8 / @testing-library/react / @testing-library/jest-dom / @testing-library/user-event / jsdom）
- [ ] 新建 `Apps/vitest.config.ts`（jsdom + setupFiles + coverage v8 不设阈值）
- [ ] 新建 `Apps/src/test/setup.ts` 与 `Apps/src/test/tauri-mock.ts`
- [ ] `Apps/.gitignore` 增加 `coverage/`
- [ ] 写一条 sanity 用例：`Apps/src/app/utils/__tests__/time.test.ts` 验证 `formatRelative` 之类的时间格式函数
- [ ] `pnpm test:web` 首次跑通（1 用例通过 = Phase 1 完成）

### Phase 2: 前端 utils + hooks 单测

- [ ] `utils/device.test.ts`：`buildDevices` / `categorizeDevices` 的分区逻辑（覆盖已配对 / 附近 / 离线三分支）
- [ ] `utils/message.test.ts`：peer:* 回应码 → 用户可见文案映射（协议契约层，回归价值高）
- [ ] `utils/settings.test.ts`：配置默认值 + 合并 + 边界
- [ ] `utils/time.test.ts`：相对时间 + 时区边界
- [ ] `hooks/usePairingCountdown.test.ts`：60s 倒计时 + 到期回调 + 配对码轮换触发时机
- [ ] `hooks/useOverlayLifecycle.test.ts`：z-index 分层的开关时序
- [ ] `pnpm test:web` 全绿；生成一次 `pnpm test:web:coverage` 记录基线到 `Plans/2026-07-11-测试基建搭建/coverage-baseline.md`

### Phase 3: Rust dev-dependencies + 单元测试

- [ ] `Apps/src-tauri/Cargo.toml` 增加 `[dev-dependencies]`：tokio(test-util) / pretty_assertions / insta / tempfile
- [ ] `crypto/keys.rs`：X25519 派生一致性 + 设备指纹稳定性 + hex 编解码
- [ ] `sync/dedup.rs`：同内容 / 不同内容 / 边界哈希；覆盖窗口过期
- [ ] `network/protocol.rs`：帧编解码往返 + `peer:*` 回应码枚举完整性（insta 快照）
- [ ] `storage/json.rs`：默认值 / 损坏 JSON 回退 / tempfile 隔离
- [ ] `clipboard/history_preview.rs`：文本摘要长度截断 + 图片/文件类型判定
- [ ] `pnpm test:rust` 首次全绿

### Phase 4: Rust 集成测试（同进程双端回环）

- [ ] 若 `ClipboardSource` trait 与 `listen_on(SocketAddr)` 不具备，先做最小抽象（提交前用 gitnexus_impact 检查）
- [ ] `tests/loopback_direct.rs`：起两个 listener → 帧收发 → 关闭；覆盖握手成功 / 密钥不匹配 / 中途断连
- [ ] `tests/pairing_handshake.rs`：6 位码生成 + 双端交换公钥 + 建立会话 + 拒绝错误配对码
- [ ] `tests/clipboard_sync_e2e.rs`：同进程双引擎 + InMemorySource，验证文本 / 图片 / 文件三条通路，包括 `peer:handled` 映射为成功
- [ ] `pnpm test:rust` 全绿；`pnpm test`（根，跑前端 + Rust）全绿

### Phase 5: 文档与收尾

- [ ] 在 `AGENTS.md` 增补一节「测试基建」说明脚本入口与目录约定（不改现有内容，仅追加）
- [ ] 在 `Docs/` 下新增 `TESTING.md`（可选）汇总 Tauri mock 用法、Rust 集成测试模板
- [ ] 记录 `coverage-baseline.md`：本次落地时前端覆盖率快照
- [ ] 更新 `COMPLETED.md` / `UNEXECUTED.md`

## 关键依赖

**前端新增：**

- `vitest ^2.1`、`@vitest/ui ^2.1`、`@vitest/coverage-v8 ^2.1`
- `@testing-library/react ^16.1`、`@testing-library/jest-dom ^6.6`、`@testing-library/user-event ^14.5`
- `jsdom ^25`

**Rust 新增（`[dev-dependencies]`）：**

- `tokio = { features = ["macros", "rt", "test-util"] }`（复用现有版本）
- `pretty_assertions = "1"`、`insta = "1"`、`tempfile = "3"`

**已有可复用：**

- Vite 8 配置（vitest 复用 resolve/alias）
- tokio 运行时（生产依赖，测试共享）
- 现有 `src-tauri/src/**` 模块（内联测试直接加）

## 风险与注意事项

| 风险 | 级别 | 缓解 |
|------|------|------|
| Vitest 与 Vite 8 版本对齐 | 低 | Vitest 2.1+ 已支持 Vite 8；锁定后 CI 之外无冲突 |
| Tauri mock 覆盖不全导致组件测试报错 | 中 | Phase 1/2 仅测 utils + hooks；组件测试延后到 mock 稳定后再上 |
| `sync::engine` / `direct` 抽象改动波及生产路径 | **高** | Phase 4 抽象改动前用 `gitnexus_impact` 检查；仅做最小 trait 化，行为 100% 保持一致；配对 / 剪贴板双机联调回归一次 |
| jsdom 无法模拟 IntersectionObserver / ResizeObserver | 低 | 在 `setup.ts` 里 stub；影响的组件测试延后 |
| insta 快照文件散落 | 低 | 用 `cargo insta review` 统一审阅；快照文件纳入 git |
| 首次 `pnpm install` 增加依赖体积 | 低 | 全部 devDependencies，不影响运行时 |
| Windows 上 vitest 观察模式的文件锁 | 低 | 使用 `test:web`（run 模式）为默认；watch 模式仅本地开发 |

## 验收标准

1. **`pnpm test`（根）一键跑通**：先跑 Vitest 再跑 `cargo test`，全绿退出码 0
2. **`pnpm test:web` ≤ 10s**：utils + hooks 用例整体在 10 秒内完成
3. **`pnpm test:rust` ≤ 30s**：单测 + 集成测试整体在 30 秒内完成（含编译缓存后）
4. **协议契约覆盖**：`peer:*` 回应码所有分支被 Rust 单测与 `utils/message.test.ts` 双向覆盖
5. **同进程双引擎联调通过**：文本 / 图片 / 文件三路在集成测试中稳定 pass 至少 20 次连跑
6. **无破坏性回归**：现有 `pnpm dev` / `pnpm check` / `pnpm build` 全部不受影响
7. **覆盖率报告可用**：`pnpm test:web:coverage` 产出 `coverage/index.html`，本次基线记录到 `coverage-baseline.md`

## 参考讨论

- [01-测试基建整体方案.md](01-测试基建整体方案.md) — 分层策略、目录与脚本命名
- [02-决策确认与E2E取舍.md](02-决策确认与E2E取舍.md) — Vitest 确定、覆盖率与 E2E 分阶段决策
- 关联历史：`Plans/归档/2026-06-04-single-machine-connection-testing/` — 手动联调方案，本方案的替代目标

## 执行顺序建议

**Phase 1 → 2 → 3 → 4 → 5**，前 3 个 Phase 强制串行，Phase 4 因涉及最小抽象改动，须在 Phase 3 单测覆盖到位后再动。

**里程碑检查点：**

- Phase 1 完成 → 有可跑的 `pnpm test:web`（1 用例）
- Phase 2 完成 → 前端覆盖率基线成文
- Phase 3 完成 → Rust 五大模块单测就位，可保护后续重构
- Phase 4 完成 → 手工双机联调可以逐步减少
- Phase 5 完成 → 主题可锁定归档

确认本方案后，说「开始执行」或「按方案做」即可进入开发；说「锁定」则在完成后归档本主题。
