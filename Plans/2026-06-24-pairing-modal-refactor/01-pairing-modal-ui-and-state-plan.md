# 第 1 轮：配对码连接面板重构方案（对比 Figma 与当前实现）

> 时间: 2026-06-24

## 用户提问

设备管理中，配对码连接面板已经做了更新，这是大概得更新总结：
配对码连接面板重构总结
我已经查看了当前的配对弹窗实现，以下是本次重构的核心修改要点：

🎯 核心改动：统一状态驱动机制
1. 内部选中状态 selectedDevice

新增内部状态 useState<NearbyDevice | null>(initialTarget ?? null)，作为配对目标的唯一数据源
支持从外部传入初始设备（initialTarget），也支持用户从列表中动态切换
2. 动态设备列表过滤

const listDevices = allDiscoverable.filter((d) => d.id !== selectedDevice?.id);
设备列表自动排除当前选中的设备，避免重复显示
当 selectedDevice 变化时，列表自动更新
3. 切换交互函数 selectFromList

点击列表中的设备时触发切换
自动完成三件事：
更新 selectedDevice 为新选中设备
重置配对流程状态为 "idle"
清空已输入的验证码
📋 功能特性
✅ 动态切换目标 — 用户可在配对过程中随时点击列表切换设备
✅ 自动回流 — 原选中设备自动回到列表中（通过 filter 实现）
✅ 状态隔离 — 切换设备时配对状态重置，每次验证都是全新流程
✅ 信息关联 — 验证时自动使用 selectedDevice 的 name/os/ip 信息
✅ UI同步 — Header 标题、设备卡片、验证流程都基于同一状态源

🔄 完整交互流程
打开弹窗
   ↓
有/无初始选中设备（selectedDevice）
   ↓
底部显示可选设备列表（排除选中项）
   ↓
用户点击列表中的设备
   ↓
触发 selectFromList → 切换 selectedDevice
   ↓
原设备回到列表，新设备成为选中目标
   ↓
配对状态重置，可重新验证
这次重构的最大优势是将多个数据源（props + 本地选择）统一为单一内部状态，配对流程、UI呈现、设备列表都基于 selectedDevice 驱动，逻辑更清晰，状态同步更可靠。

另外还加了倒计时进度条，进度条到10s以下是会显示红色并闪烁，

你根据以上总结，再参考figma上相关界面，然后对比当前界面，给一个修改方案

## 背景与分析

### 代码现状（讨论时点）

- `PairingModal.tsx` 仍为旧版扁平结构：状态条 → 本机码 → 手输对方码 → 带「连接设备」按钮的完整设备列表。
- 仓库中尚未出现 `selectedDevice`、`selectFromList`、`initialTarget`、倒计时进度条等实现。
- `usePairingFlow.ts` 使用 `pairingTargetName: string | null`，仅保存设备名称，非完整 `Device` 对象。
- 设备页「建立连接」直接调用 `handleConnectLan`，未以 `initialTarget` 打开弹窗。

### Figma 对齐基准

- 文件：`PlanarClip`，页面：`设备`
- 最新 `PairingModal` 节点：`46:1694`（有选中设备 + 倒计时 urgent 态）
- 无选中设备基准：`46:621`
- 弹窗宽度约 **360px**；本机码标签为「本机配对码」；分隔文案为「或输入对方的验证码」；提交按钮为「验证」。

### 当前实现 vs Figma / 重构目标

| 维度 | 当前实现 | Figma 最新 + 重构目标 |
|------|----------|----------------------|
| 标题 | 固定「连接新设备」 | 无选中：`连接新设备`；有选中：`连接 {设备名}` |
| 副标题 | 「通过配对码与设备列表建立连接」 | 有选中：`完成验证码配对以建立信任连接` |
| 选中设备 | 无独立卡片 | Header 下展示 OS 图标 + 名称 + IP:端口 |
| 状态条 | 多 stage 图标 + 长文案 | 精简为「等待验证」+ WiFi 图标 |
| 本机码标签 | 「你的配对码」 | 「本机配对码」 |
| 倒计时 | 无 | 码下方 3px 进度条；≤10s 变红并闪烁 |
| 码样式 | 始终 primary | 正常 cyan；urgent 时 `#ef4444` + 红色描边 |
| 提示文案 | 「请在另一台设备上输入此配对码」 | 「请在对方设备的 PlanarClip 中输入此验证码」 |
| 分隔 | 「或」 | 「或输入对方的验证码」 |
| 提交按钮 | 「发起配对 / 开始配对」 | 「验证」 |
| 设备列表 | 全部展示 + 「连接设备」按钮 | 排除选中项；整行可点切换；右侧 PlugZaz 图标 |
| 弹窗宽度 | `max-w-sm`（384px） | 360px |

## 建议与回答

### 推荐架构：两层状态分离

- **Modal 层**：`selectedDevice` 作为 UI 唯一数据源；`listDevices = allDiscoverable.filter(id ≠ selected)`；`selectFromList` 负责切换并重置上层流程。
- **App / usePairingFlow 层**：继续管理 `pairingStage`、Tauri 命令、连接生命周期；将 `pairingTargetName` 升级为 `pairingTarget: Device | null`。

### 分阶段修改

**阶段 1 — 重构 PairingModal 布局**

建议拆分子组件：`PairingModalHeader`、`SelectedDeviceCard`、`PairingStatusBar`、`LocalPairingCode`、`RemoteCodeInput`、`SwitchableDeviceList`。

Props 要点：

- `initialTarget?: Device | null`
- `allDiscoverable: Device[]`
- `onSelectDevice(device)` — 切换目标并重连

布局顺序（对齐 Figma `46:1694`）：

1. Header（动态标题 + 关闭）
2. `[条件]` SelectedDeviceCard
3. PairingStatusBar（`awaiting_code` / `incoming_pairing` / `submitting_code`）
4. LocalPairingCode + PairingCountdownBar
5. RemoteCodeInput
6. SwitchableDeviceList

移除：顶部通用多态状态条、列表内「连接设备」按钮。

**阶段 2 — 改造 usePairingFlow 与入口**

- `openPairingModal(initialTarget?)`：「+」无参；设备卡片传入 device。
- `switchPairingTarget(device)`：若在连接中先 `disconnect` → 清 input/error → stage=`idle` → 再 `connect_lan`。
- `DevicesPage` 点击「建立连接 / 重新连接」改为打开弹窗并带入 `initialTarget`。

**阶段 3 — 倒计时进度条（初版假设，见第 2 轮修正）**

- 总时长 60s（与 `direct.rs` 握手超时一致）。
- 启动：`awaiting_code`、`incoming_pairing`。
- ≤10s：红色 + `animate-pulse`。
- 初版曾建议归零后报错；**已在第 2 轮修正为轮换配对码并重置倒计时**。

**阶段 4 — 文案与样式**

见上表；弹窗 `max-w-[360px]`。

**阶段 5 — 入站流程边界**

`incoming_pairing` 不参与 `selectedDevice` 列表切换；列表只读或隐藏；仍由 `IncomingConnectionPrompt` 处理确认。

### 实施检查表

- [ ] Modal 内 `selectedDevice` + 列表过滤
- [ ] `selectFromList` 切换并重置流程
- [ ] Header / 选中卡片联动
- [ ] 列表改为可切换行，去掉「连接设备」
- [ ] 60s 倒计时进度条，≤10s 红色闪烁
- [ ] 文案与 Figma 一致
- [ ] `pairingTarget` 从 string 升级为 Device
- [ ] 设备页入口带入 `initialTarget`
- [ ] 入站配对不受列表切换影响
- [ ] 切换设备时正确 disconnect + 重连

### 风险

1. 切换设备必须先 `disconnect`，否则后端仍绑定旧会话。
2. 倒计时语义需与后端配对码机制对齐（见第 2 轮）。
3. 「+」与列表点击语义不同：前者无选中，后者选中并发起连接。

## 本轮要点

- 确定以 Figma `46:1694` 为 UI 真源，重构 `PairingModal` 为 `selectedDevice` 驱动的状态机 UI。
- 业务状态仍由 `usePairingFlow` 管理，Modal 只管呈现与切换交互。
- 设备列表从「连接按钮」改为「切换目标行」；选中设备提升到 Header 下方独立卡片。
- 倒计时进度条为必做项，≤10s urgent 态；归零行为留待第 2 轮明确。
