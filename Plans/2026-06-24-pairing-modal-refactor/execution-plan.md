# 配对码连接面板重构 — 待执行方案

> 生成时间: 2026-06-24
> 基于讨论: [01-pairing-modal-ui-and-state-plan.md](01-pairing-modal-ui-and-state-plan.md) | [02-countdown-rotate-pairing-code.md](02-countdown-rotate-pairing-code.md)

## 需求概述

重构设备管理中的「配对码连接」弹窗（`PairingModal`），对齐 Figma「设备」页最新设计（节点 `46:1694` / `46:621`），并实现：

1. **统一状态驱动**：Modal 内 `selectedDevice` 作为 UI 唯一数据源，列表动态过滤、可切换目标设备。
2. **视觉与交互对齐**：动态 Header、选中设备卡片、精简状态条、本机码 + 倒计时进度条、可切换设备列表。
3. **倒计时语义**：60s 进度条，≤10s 红色闪烁；**归零时仅轮换配对码并重置倒计时**，不断连、不报错。

## 技术决策

| 决策项 | 选择 | 理由 | 来源 |
|--------|------|------|------|
| UI 真源 | Figma `PairingModal` `46:1694` | Design 为视觉与结构依据 | 第 1 轮 |
| 选中状态位置 | Modal 内 `selectedDevice` + App 层 `pairingTarget: Device` | UI 与连接生命周期分离 | 第 1 轮 |
| 列表交互 | 整行点击切换目标，排除选中项 | 与重构总结、Figma 一致 | 第 1 轮 |
| 弹窗宽度 | `max-w-[360px]` | 对齐设计稿 | 第 1 轮 |
| 倒计时时长 | 60s | 与 `direct.rs` 握手窗口一致 | 第 1、2 轮 |
| urgent 阈值 | ≤10s，红色 + 闪烁 | 产品要求 + Figma urgent 态 | 第 1、2 轮 |
| 归零行为 | 轮换配对码 + 重置 60s | 用户明确修正 | 第 2 轮 |
| 配对码机制 | 会话级动态码 + `rotate_pairing_code` | 静态指纹码无法真正轮换 | 第 2 轮 |
| 入站配对 | 不参与列表切换 | 避免与 `IncomingConnectionPrompt` 冲突 | 第 1 轮 |
| 轮换范围 | 仅 `awaiting_code` / `incoming_pairing` | 避免 idle 时码无故变化 | 第 2 轮 |

## 架构设计

```text
┌─────────────────────────────────────────────────────────┐
│ App.tsx / usePairingFlow                                 │
│  pairingStage, pairingInput, pairingTarget (Device)       │
│  connect_lan / submit_pairing_code / disconnect           │
└───────────────────────┬─────────────────────────────────┘
                        │ initialTarget, allDiscoverable
                        │ onSelectDevice → switchPairingTarget
                        ▼
┌─────────────────────────────────────────────────────────┐
│ PairingModal                                             │
│  selectedDevice (internal)                               │
│  listDevices = all.filter(id ≠ selected)                 │
│  selectFromList → reset + onSelectDevice                 │
│  usePairingCountdown → onExpire → rotate_pairing_code    │
└───────────────────────┬─────────────────────────────────┘
                        │ Tauri commands / events
                        ▼
┌─────────────────────────────────────────────────────────┐
│ Rust AppState                                            │
│  active_pairing_code, pairing_code_issued_at             │
│  responder_verify_code: 超时 → rotate → 继续等待         │
│  emit pairing-code-rotated                               │
└─────────────────────────────────────────────────────────┘
```

### 组件拆分（建议）

```text
PairingModal/
├── PairingModal.tsx
├── PairingModalHeader.tsx
├── SelectedDeviceCard.tsx
├── PairingStatusBar.tsx
├── LocalPairingCode.tsx      # 含 PairingCountdownBar
├── RemoteCodeInput.tsx
└── SwitchableDeviceList.tsx
```

## 实现步骤

### Phase 1: 后端 — 会话级配对码

- [ ] `AppState` 增加 `active_pairing_code`、`pairing_code_issued_at`
- [ ] 入站陌生设备配对开始时生成首个会话码
- [ ] `get_pairing_code` 返回当前会话码（有会话时）
- [ ] 新增 `rotate_pairing_code` 命令
- [ ] 修改 `responder_verify_code`：60s 超时 → 轮换 → 继续等待（循环），仅取消/断连时结束
- [ ] 新增事件 `pairing-code-rotated { code, expires_at_ms }`

### Phase 2: 前端 — PairingModal UI 重构

- [ ] 按 Figma 重排布局与子组件
- [ ] 实现 `selectedDevice`、`listDevices`、`selectFromList`
- [ ] 动态 Header / SelectedDeviceCard / 精简状态条
- [ ] 文案对齐：「本机配对码」「验证码」「验证」等
- [ ] 列表改为可切换行（PlugZap），移除「连接设备」按钮
- [ ] 弹窗宽度 `max-w-[360px]`

### Phase 3: 前端 — 流程与入口打通

- [ ] `pairingTargetName` → `pairingTarget: Device | null`
- [ ] `openPairingModal(initialTarget?)`、`switchPairingTarget(device)`
- [ ] `DevicesPage` 连接按钮带入 `initialTarget`
- [ ] 切换设备时 `disconnect` + 重连 + 重置 input/error/stage
- [ ] 入站 `incoming_pairing` 隐藏/禁用列表切换

### Phase 4: 倒计时与轮换联调

- [ ] 实现 `usePairingCountdown`（60s，≤10s urgent）
- [ ] `onExpire` → `rotate_pairing_code` + 重置，不设 error
- [ ] 订阅 `pairing-code-rotated` 同步 `pairingCode`
- [ ] urgent 态：进度条 + 数字 + 描边 `#ef4444` + `animate-pulse`
- [ ] 轮换提示文案；输旧码与输错码区分

### Phase 5: 验收

- [ ] 双机：选中设备、切换列表、验证成功
- [ ] 等满 60s：码变、进度条回满、连接未断
- [ ] ≤10s 红色闪烁
- [ ] 取消/关闭/真断连仍正常
- [ ] 浏览器预览模式降级合理

## 关键依赖

| 层级 | 文件 / 模块 |
|------|-------------|
| UI | `PairingModal.tsx`、`DevicesPage.tsx`、`App.tsx` |
| 流程 | `usePairingFlow.ts`、`useConnectionBridge.ts` |
| 后端 | `lib.rs`（AppState、commands）、`network/direct.rs`（握手） |
| 设计 | Figma `PlanarClip` / 页面 `设备` / `PairingModal` |

## Figma 参考节点

| 场景 | 节点 ID |
|------|---------|
| 有选中设备 + urgent 倒计时 | `46:1694` |
| 无选中 + 列表可切换 | `46:621` |
| 旧版弹窗（对比用） | `1:1529` |

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| 切换设备时会话未释放 | `switchPairingTarget` 必先 `disconnect` |
| 前后端倒计时漂移 | 以 `pairing-code-rotated` 事件为准同步 |
| 轮换后用户仍输旧码 | 明确文案 + `invalid_code` 提示查看新码 |
| 静态指纹码与动态码混用 | 有活跃握手时统一走会话码 |

## 参考讨论

- [第 1 轮：UI 与状态方案](01-pairing-modal-ui-and-state-plan.md)
- [第 2 轮：倒计时轮换语义](02-countdown-rotate-pairing-code.md)
