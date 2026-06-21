# 第 1 轮：讨论 UI 优化与 Figma 项目融入方案

> 时间: 2026-06-21 16:31

## 用户提问

新建一个Plan主题，讨论界面UI的优化，目前我通过figma生成了一个前端项目 `c:\Users\Administrator\Documents\Workspace\planarclip\Resources\PlanarClip` ，你需要将该项目融入当前项目中，你可以用figma创建的项目直接整个替换掉当前项目UI，然后将rust后端功能重新连接到该新的UI上

## 背景与分析

### 主题判断

- `Plans/` 下最新主题是 `2026-06-18-[LOCKED]-vite-plus-adoption`，已锁定，因此本次讨论按新主题创建。
- 本主题与既有 LAN 连接主题、Vite Plus 采纳主题都不属于同一讨论目标：这次目标是“整体替换桌面端 UI，并把现有 Rust/Tauri 后端重新接回新界面”。
- 依据链式继承规则，已从上一锁定主题复制 `UNEXECUTED.md`。其中仅有一条遗留项：`在建立测试基线后补 vp test`。它与本次 UI 融合不是直接阻塞关系，默认继续保留为跨主题工程化待办，不纳入本主题主线。

### 当前项目现状

#### 1. 现有桌面应用结构

- 当前主应用位于 `Apps/planarclip`。
- 前端不是 React，而是一个非常轻量的单文件页面：`Apps/planarclip/src/main.ts` 直接通过 DOM id 绑定按钮、状态区、弹层和设备列表。
- 构建链目前是 `vite-plus`，而不是标准 React + Vite。
- Tauri 入口仍然稳定，Rust 后端位于 `Apps/planarclip/src-tauri/src`。

#### 2. 当前 Rust/Tauri 已暴露给前端的能力

现有前端实际依赖的命令与事件范围很明确：

- Commands
  - `get_status`
  - `get_pairing_code`
  - `pair`
  - `get_lan_devices`
  - `connect_lan`
  - `submit_pairing_code`
  - `reject_connection`
  - `disconnect`
- Events
  - `lan-devices-changed`
  - `connection-request`
  - `connection-established`
  - `connection-failed`
  - `connection-ended`
  - `pairing-code-needed`

这说明：当前 UI 主要覆盖“局域网设备发现、连接发起、配对码输入、连接状态反馈”这条主链路。

#### 3. 当前后端数据边界

- `ClipboardSnapshot` 目前只支持 `Text(String)` 与 `Empty`，还没有图片/文件传输的数据结构。
- 后端没有现成的“剪贴板历史列表 API”“最近同步记录 API”“设备详情统计 API”。
- 配置层 `AppConfig` 目前只存：`device_name`、`key_pair`、`paired_peer`、`tcp_port`、`lan_enabled`、`trusted_peers`。
- 因此，Figma UI 里那些完整的剪贴板历史、设备状态细节、设置开关，并不能直接零改动接上；中间需要一层新的前端状态模型，必要时还要补 Rust 命令或事件。

### Figma 生成项目现状

- Figma 项目位于 `Resources/PlanarClip`。
- 它是独立的 React + Vite + Tailwind 4 风格项目，入口是 `src/main.tsx`，根组件是 `src/app/App.tsx`。
- 该 UI 不是简单卡片页面，而是完整桌面仪表盘：左侧导航、中央剪贴板历史、右侧设备面板、设置页、主题切换、明暗模式切换。
- 目前 `App.tsx` 主要依赖 `DEVICES` 与 `INITIAL_CLIPS` 等 mock 数据驱动，尚未接入 Tauri `invoke/listen`。
- Figma 项目依赖栈与当前应用差异较大：React、Tailwind 4、`@vitejs/plugin-react`、大量 Radix/shadcn 风格依赖都会进入当前应用。

### 关键差距与风险

#### 1. 窗口尺寸完全不匹配

- 当前 Tauri 主窗口尺寸是 `380 x 520`。
- Figma UI 明显按宽桌面布局设计，至少需要更宽的窗口，或者必须做响应式收缩。
- 这不是样式小修，而是产品形态变化：从“小面板工具”变成“桌面控制台”。

#### 2. 数据模型不匹配

- Figma UI 默认展示“历史剪贴板流 + 设备列表 + 设置面板”。
- 现有后端真实可提供的数据，主要是“连接状态、配对码、LAN 设备列表、连接事件”。
- 如果直接硬替换 UI 而不补适配层，新界面会像被雷劈中的漂亮空壳：能发光，但没有魂。

#### 3. 技术栈切换有连锁影响

- 当前应用是 `vite-plus + 原生 DOM`。
- Figma 项目是 `vite + React + Tailwind 4`。
- 真正要融合，基本等价于“保留 Tauri/Rust 后端，重建前端壳层与构建链”。

## 建议与回答

我的建议是：

### 推荐总体策略

**把 Figma 项目作为新的前端 UI 基线，整体替换当前前端层；但不要替换 `src-tauri` 和 Tauri 工程骨架。**

也就是说，执行层面应当是：

- 保留 `Apps/planarclip/src-tauri` 作为后端与桌面宿主；
- 用 Figma 生成的 React UI 替换 `Apps/planarclip` 现有前端页面与样式体系；
- 再在 React 里重新接回现有 Rust commands / events；
- 最后补上新 UI 需要、但当前后端还缺失的数据接口。

这个策略比“整个项目直接用 Figma 目录覆盖”更安全，原因是：

- Figma 项目并不包含你当前已经写好的 Rust/Tauri 能力；
- 现有桌面壳、托盘逻辑、发现/连接/同步主链路都在 `src-tauri`；
- 真正该被整体替换的是 **前端 UI 层**，不是整套桌面应用工程。

### 建议拆成两层迁移

#### Layer A：先完成“视觉壳替换 + 真实连接主链路接通”

目标是尽快把新 UI 跑起来，并让核心功能不失联：

- React 入口接管现有窗口；
- 剪掉旧 `index.html + src/main.ts` 的 DOM 驱动方式；
- 在新 UI 中先打通：
  - 当前连接状态
  - 本机配对码
  - 局域网设备列表
  - 发起连接
  - 输入配对码
  - 拒绝连接
  - 断开连接
  - 后端事件驱动的状态更新

这样即便“剪贴板历史”“高级设置面板”先部分占位，产品也已经从旧 UI 成功迁移到新 UI。

#### Layer B：再补“Figma 仪表盘所需数据能力”

针对新 UI 比旧 UI 多出来的能力，再逐步补接口：

- 最近剪贴板历史（至少文本历史）
- 当前连接设备更完整的状态信息
- 设备名称 / 端口 / LAN 开关等设置读写
- 最近同步时间、最近一次来源设备等摘要信息
- 后续如果真的要支持图片/文件传输，再扩展 `ClipboardSnapshot` 与协议层

### 推荐的落地方式

#### 1. 前端目录策略

建议把 Figma 项目的以下内容迁入当前主应用：

- `src/app/**`
- `src/styles/**`
- `src/main.tsx`
- 必要的 `index.html`
- React/Tailwind/Vite 配置与依赖

但不要直接把 `Resources/PlanarClip` 原样当成最终可运行应用目录，因为当前真正的产品入口仍是 `Apps/planarclip`。

#### 2. 前端状态适配层

建议新增一个明确的“前端适配层”概念，由它负责：

- 把 Tauri `invoke` 的返回值转换成 React state；
- 把 Tauri `listen` 事件转换成 UI store 更新；
- 把 Figma 里的 mock 数据结构改造成真实数据结构；
- 对缺失后端能力的面板先提供占位状态，而不是伪造假数据。

这样可避免把 `invoke/listen` 逻辑散落在巨大 `App.tsx` 中。

#### 3. UI 信息架构建议

Figma 的三栏桌面布局是可用的，但建议按当前后端能力做一期收敛：

- `剪贴板` 页面：先展示“最近同步摘要 + 文本历史（若补接口）/ 临时空态”
- `设备` 页面：优先承接当前的 LAN 发现、连接、配对、已配对设备展示
- `设置` 页面：优先放真实可控项，例如设备名、端口、LAN 开关、主题模式

不要一上来把所有 mock 卡片都强行接真数据，否则容易把范围吹成热气球，飞得很高，落不了地。

#### 4. 窗口策略

建议在本主题执行时同步调整桌面窗口配置：

- 默认窗口尺寸改成更适合桌面控制台的宽高；
- 允许最小尺寸限制，避免三栏布局被压碎；
- 如仍希望保留“小窗工具感”，则需要额外设计响应式折叠布局，而不是直接套用 Figma 桌面稿。

### 本轮推荐结论

本次主题的推荐方案是：

1. **新开独立 Plan 主题处理 UI 全量替换与回接。**
2. **执行时仅整体替换前端 UI 层，不替换 Rust/Tauri 后端工程。**
3. **优先打通现有命令/事件主链路，再补 Figma 仪表盘缺失的数据接口。**
4. **将窗口尺寸与响应式策略视为本主题的必做项，而不是附属项。**
5. **将“图片/文件剪贴板历史”明确列为后续扩展，不默认纳入首轮 UI 替换。**

## 本轮要点

- 本次讨论已确认为新主题：`2026-06-21-ui-refresh-figma-integration`
- 当前真正该替换的是 `Apps/planarclip` 的前端 UI 层，而不是整个 Tauri/Rust 工程
- Figma UI 是完整 React 仪表盘，当前后端只覆盖连接主链路，数据能力并不对齐
- 首轮执行建议分两层：先迁 UI 壳与连接流程，再补历史列表与设置接口
- 现有 Tauri 窗口尺寸与新 UI 形态冲突，必须纳入本主题设计边界
