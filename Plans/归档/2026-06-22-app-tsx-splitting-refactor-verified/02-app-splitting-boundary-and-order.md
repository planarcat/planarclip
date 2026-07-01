# 第 2 轮：细化 App.tsx 拆分边界、落地顺序与验证点

> 时间: 2026-06-22 10:28

## 用户提问

可以

## 背景与分析

本轮是在上一轮基础上继续收敛“如何拆分 `Apps/planarclip/src/app/App.tsx`”的执行边界。上一轮已经明确：

- 当前问题核心是 **职责混杂**，而不是单纯的行数过长；
- 第一轮实施应以 **无行为变化** 为硬约束；
- 推荐按 **类型/常量/纯函数 → 视图组件 → hooks / 桥接逻辑** 的顺序推进。

继续细化后，我认为这次拆分最关键的是先回答三个问题：

1. **哪些东西适合第一批拆？**
2. **哪些东西不适合一开始就拆？**
3. **拆完后如何验证没有把连接链路拆坏？**

### 一、第一批可直接拆出的内容

这些内容的共同特点是：**低副作用、边界清晰、搬家成本低**。

#### 1) 类型与 payload 模型

当前顶部类型可以优先抽到 `src/app/types.ts`：

- `OS`
- `NavId`
- `DeviceStatus`
- `ViewMode`
- `ColorScheme`
- `ClipType`
- `AppConnectionStatus`
- `PairingStage`
- `Device`
- `ClipEntry`
- `ThemeColor`
- `UiSettingsPayload`
- `SettingAvailability`
- `ClipboardHistoryPayload`
- `LanDevicePayload`
- `ConnectionRequestPayload`
- `ConnectionEstablishedPayload`
- `ConnectionFailedPayload`
- `ConnectionEndedPayload`
- `ConnectedPeer`

这些定义全部继续留在 `App.tsx` 的收益很低，只会让入口文件像一本把目录页和正文糊在一起的书。

#### 2) 主题常量与默认值

建议抽到 `src/app/constants/theme.ts`：

- `THEME_COLORS`
- `DEFAULT_UI_SETTINGS`
- `PREVIEW_UI_SETTINGS_KEY`

这里也可以顺手提供：

- `getThemeById`
- `normalizeColorScheme`

这样主题相关内容会形成单独边界，后面如果要做主题扩展或多配色，不会继续污染 App 主容器。

#### 3) 纯工具函数

建议拆成几类：

- `src/app/utils/time.ts`
  - `relativeTime`
  - `formatTime`

- `src/app/utils/clipboard.ts`
  - `formatClipSize`
  - `mapClipboardHistory`

- `src/app/utils/device.ts`
  - `inferOs`
  - `createDeviceId`
  - `buildDevices`

- `src/app/utils/message.ts`
  - `rawMessage`
  - `normalizeUserMessage`

- `src/app/utils/settings.ts`
  - `loadPreviewUiSettings`
  - `savePreviewUiSettings`
  - `isDarkActive`
  - `applyColorScheme`
  - `applyThemeColor`

这里面除了 `settings.ts` 会接触 `window` / `document`，其余基本都属于纯函数，拆出去几乎没有争议。

#### 4) 小型通用组件

建议放到 `src/app/components/common/`：

- `OsIcon.tsx`
- `StatusDot.tsx`
- `ClipTypeIcon.tsx`
- `CopyButton.tsx`
- `SettingBadge.tsx`
- `ThemeSwatch.tsx`

它们具备共性：

- 逻辑独立；
- props 简单；
- 不直接依赖 Tauri 状态流；
- 被多个页面复用或有复用潜力。

#### 5) 页面级 / 区域级组件

建议放到：

- `src/app/components/layout/Sidebar.tsx`
- `src/app/components/layout/DevicesPanel.tsx`
- `src/app/components/pages/ClipboardPage.tsx`
- `src/app/components/pages/DevicesPage.tsx`
- `src/app/components/pages/SettingsPage.tsx`
- `src/app/components/overlays/PairingModal.tsx`

这批组件其实已经具备完整的独立文件条件。第一轮只要保持 props 不变，拆分风险很低。

### 二、不建议一开始就大拆的内容

下面这些逻辑虽然也该拆，但**不建议在第一批就同时大改**。

#### 1) `useEffect` 里的连接桥接主链路

当前 `App` 最重的部分，是后半段这条链路：

- 启动时并行拉取 `get_status / get_pairing_code / get_lan_devices / get_ui_settings / get_clipboard_history`
- 注册多个 `listen(...)`
- 按事件更新 `connectedPeer / status / pairingStage / lastMessage / clips / lanDevices`
- 维护定时轮询 `refreshConnectionStatus`

这部分当然适合以后抽成 `useConnectionBridge`，但如果在第一轮就抽，很容易带来两个副作用：

- hook 的入参和返回值会一下子膨胀；
- 连接状态、配对状态、UI 状态会被硬拆成多段，导致理解成本反而上升。

所以第一轮更稳妥的做法是：**暂时保留主桥接逻辑在 App 内，但把它上下游依赖先清空。**

也就是先把：

- 类型
- 工具函数
- 页面组件
- 小组件

都移出去，等 App 文件自然瘦下来以后，再观察桥接逻辑能否自然收敛成 1~2 个 hooks，而不是靠意志硬切。

#### 2) 配对流程状态机

下面这些状态当前高度关联：

- `showPairing`
- `pairingInput`
- `pairingStage`
- `pairingTargetName`
- `pairingHelperText`
- `pairingError`
- `incomingRequest`

以及这些动作：

- `resetPairingFlow`
- `closePairingModal`
- `handleManualPair`
- `handleConnectLan`
- `handleSubmitPairingCode`
- `handleRejectIncoming`
- `handleDisconnect`

这是一套隐式状态机。它最终确实适合抽成 `usePairingFlow`，但第一轮就抽的话，最容易出现“文件更优雅了，行为却悄悄变了”的问题。

我的建议是：

- **第一轮：保留在 App**
- **第二轮：等页面和工具函数拆完后，再把这组状态和动作一并抽到 hook**

这样可以避免“状态留一半在 App，动作抽一半到 hook”这种最难维护的半吊子结构。

### 三、推荐的两阶段实施边界

#### Phase A：先做结构拆分，不碰状态流

这个阶段只做三件事：

1. 抽 `types / constants / utils`
2. 抽 `common components / page components / layout components / overlay`
3. App.tsx 改成只负责：
   - 维护 state
   - 定义 handlers
   - 组装页面

这个阶段的目标不是“完美架构”，而是把 App.tsx 从“巨型厨房”先整理成“还像个客厅”。

#### Phase B：再做逻辑拆分

当 Phase A 稳定以后，再做：

1. 抽 `useUiTheme`
2. 抽 `useConnectionBridge`
3. 评估是否抽 `usePairingFlow`
4. 抽 `services/tauri.ts` 或类似轻量桥接文件，统一 `callCommand` / `listen` 周边封装

这里我强调“评估是否抽 `usePairingFlow`”，因为它不一定必须独立成 hook；如果拆完发现它和 `useConnectionBridge` 强依赖，也可能更适合做成同一 hook 内部的子模块，而不是为了对称而拆。

### 四、可以直接照着走的文件边界清单

下面给出更接近执行用的边界建议：

| 目标文件 | 建议承载内容 | 本轮是否优先 |
|---|---|---|
| `src/app/types.ts` | 所有 app 内部领域类型与 payload 类型 | 是 |
| `src/app/constants/theme.ts` | 主题色、默认设置、主题查找/规范化 | 是 |
| `src/app/utils/time.ts` | 时间格式化 | 是 |
| `src/app/utils/clipboard.ts` | 剪贴板历史映射与大小格式化 | 是 |
| `src/app/utils/device.ts` | 设备推断与设备列表构建 | 是 |
| `src/app/utils/message.ts` | 错误文案归一化 | 是 |
| `src/app/utils/settings.ts` | 浏览器预览态设置读写、主题应用 | 是 |
| `src/app/hooks/useRelativeTicker.ts` | 相对时间刷新 hook | 是 |
| `src/app/components/common/*` | 通用小组件 | 是 |
| `src/app/components/layout/*` | 侧栏、右侧设备概览栏 | 是 |
| `src/app/components/pages/*` | 剪贴板、设备、设置三页 | 是 |
| `src/app/components/overlays/PairingModal.tsx` | 配对弹层 | 是 |
| `src/app/hooks/useUiTheme.ts` | 主题状态与持久化逻辑 | 下一阶段 |
| `src/app/hooks/useConnectionBridge.ts` | Tauri 初始化、事件监听、状态桥接 | 下一阶段 |
| `src/app/hooks/usePairingFlow.ts` | 配对状态机与相关操作 | 下一阶段 |

### 五、实施时的风险控制点

为了防止“拆分以后看着很整齐，实际上悄悄坏了”，我建议把回归点写死，不要靠感觉。

#### 必测交互

1. **启动初始化**
   - 桌面端启动后是否还能正确加载：
     - 状态
     - 配对码
     - LAN 设备
     - UI 设置
     - 剪贴板历史

2. **主题设置**
   - 切换背景模式
   - 切换主题色
   - 浏览器预览模式下本地暂存是否仍正常
   - 桌面端保存设置是否仍正常

3. **设备连接主链路**
   - 点击局域网设备发起连接
   - 收到 `pairing-code-needed`
   - 输入配对码
   - 建立连接
   - 断开连接

4. **来连请求链路**
   - 收到 `connection-request`
   - 展示弹层
   - 拒绝连接

5. **连接异常链路**
   - 连接失败
   - 连接结束
   - 文案是否仍然自然中文

6. **剪贴板展示**
   - 历史列表/网格切换
   - 空态显示
   - 复制按钮
   - `clipboard-history-changed` 事件后 UI 是否更新

#### 轻量验证方式

当前阶段不建议一上来补全量测试，而是沿用已有最小验证链：

- `pnpm check:web`
- `pnpm build:web`
- `pnpm check`
- 必要时再做桌面端手工回归

### 六、我建议的最终实施姿势

如果这次要落成执行方案，我推荐的策略是：

- **本次主题先只承诺完成 Phase A + 视情况推进 Phase B 的第一步**
- 不要在一个主题里同时追求：
  - App.tsx 从 2000 行拆到极致
  - 建立完整 hooks 架构
  - 顺便补测试基线
  - 顺便做状态管理升级

这样目标会失焦。

更稳的主题目标应该是：

> 先把 `App.tsx` 从“单文件大杂烩”拆成“有明确目录边界的容器文件”，并在不改变现有行为的前提下，为后续继续抽 hook 做准备。

## 建议与回答

我建议把这次重构方案先定成一个 **两阶段计划**：

### 推荐方案

#### 阶段 1：结构拆分
- 先拆 `types / constants / utils / components / useRelativeTicker`
- 保持 App 里的状态、handlers、事件监听逻辑不变
- 目标是快速显著降低 App.tsx 体积，同时不引入行为风险

#### 阶段 2：逻辑收敛
- 视阶段 1 落地后的剩余体量，再决定：
  - 抽 `useUiTheme`
  - 抽 `useConnectionBridge`
  - 是否抽 `usePairingFlow`

### 我推荐的执行优先级

1. `types.ts`
2. `constants/theme.ts`
3. `utils/*`
4. `hooks/useRelativeTicker.ts`
5. `components/common/*`
6. `components/layout/*`
7. `components/pages/*`
8. `components/overlays/PairingModal.tsx`
9. 再评估 hooks 抽离

### 不推荐的做法

- 不推荐第一刀就把所有 `useEffect + handlers` 全抽成 hooks
- 不推荐第一轮引入 Zustand / Redux / Context 重构状态层
- 不推荐为了拆分而引入新的全局目录体系，导致改动面失控

## 本轮要点

- 已把拆分边界细化到文件级别
- 已明确第一批优先项：`types / constants / utils / components / useRelativeTicker`
- 已明确暂缓项：`useConnectionBridge / usePairingFlow`
- 已整理出一组固定回归清单，避免结构重构后行为偷偷回退
- 当前已经具备生成待执行方案的条件
