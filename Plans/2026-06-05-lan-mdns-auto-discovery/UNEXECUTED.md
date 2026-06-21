# 未执行 / 遗留事项

> 生成时间: 2026-06-21
> 来源主题: 2026-06-05-lan-mdns-auto-discovery
> 继承自: 2026-05-26-cross-platform-clipboard-sync
> 说明: 本文档记录当前主题执行到 Phase 3 后仍未完成、未处理或计划未来处理的事项

---

## 📋 继承事项的处理结果

| 事项 | 处理结果 | 说明 |
|:---|:---|:---|
| 单机双实例集成测试 | 移出传递 | 已在 6.4 主题中确认丢弃，不再作为后续 LAN 主题的遗留事项 |
| Phase 4: mDNS LAN 自动发现 | 改写并吸收 | 已被当前 6.5 主题展开并推进到 Phase 3，后续以 Phase 3 / 4 的剩余事项继续推进 |
| 其他继承的未来能力 | 保留 | 与当前 LAN 收尾不冲突，继续保留到后续主题 |

---

## ⏳ 仍待处理的遗留事项

### ❌ 执行失败

（无）

### ⚠️ 未开发完成 / 完成度不足

| 事项 | 当前状态 | 差距描述 | 后续建议 |
|:---|:---|:---|:---|
| `trusted_peers` 自动接受的重启后表现验证 | 部分完成 | 代码路径与自动化测试已覆盖主链路，但还没有完成“首次配对后重启双端再连接”的真实设备验收 | 在 Phase 3 继续加入双端重启后的真实联调步骤 |
| LAN 异常路径验证 | 部分完成 | 当前已有自动化测试覆盖错误配对码，但主动拒绝、超时、对端退出仍未形成稳定自动回归，也未完成真机覆盖 | 在 Phase 3 中优先补齐真机异常路径，并视需要单独补测试夹具 |
| 文本双向同步、去重与断线状态回落验收 | 未开始 | 本轮聚焦握手验证，尚未对剪贴板双向同步和断线恢复做自动或真机验收 | 在 Phase 3 中完成两端复制 / 断线 / 重连场景验证 |
| 局域网模式真机联调 | 未开始 | 当前已完成 `cargo test`、`cargo check` 与前端检查，但尚未完成双设备同局域网验收 | 在 Phase 3 中完成发现、首次配对、再次连接、双向同步与断线恢复验证 |

### 📅 被移出本次开发节奏

（无）

### 🔮 计划在未来版本加入

| 事项 | 计划版本/Phase | 参考讨论 |
|:---|:---|:---|
| 6.5 Phase 4：计划文档收尾与主题归档准备 | Phase 4 | [execution-plan.md](execution-plan.md) |
| Phase 2: SQLite 存储（clipboard_entries + settings + peers + key_store） | 后续版本 | 2026-05-26 主题 execution-plan.md |
| Phase 2: Noise IK 应用层加密 + ChaCha20-Poly1305 传输 | 后续版本 | 2026-05-26 主题 execution-plan.md |
| Phase 2: 图片同步（arboard::get_image, ≤5MB） | 后续版本 | 2026-05-26 主题 execution-plan.md |
| Phase 2: CI: GitHub Actions Win x64 + macOS ARM64 | 后续版本 | 2026-05-26 主题 execution-plan.md |
| Phase 3: 文件同步 + 大文件分块传输 | 后续版本 | 2026-05-26 主题 execution-plan.md |
| Phase 3: N 对等点 WebRTC 动态连接管理 | 后续版本 | 2026-05-26 主题 execution-plan.md |
| Phase 3: 断线自动重连 + TURN 回退 | 后续版本 | 2026-05-26 主题 execution-plan.md |
| Phase 3: 离线队列 + 冲突解决 | 后续版本 | 2026-05-26 主题 execution-plan.md |
| Phase 4: 完整 React 前端 | 后续版本 | 2026-05-26 主题 execution-plan.md |
| Phase 4: 原生通知 + 暗色主题 + 快捷键 | 后续版本 | 2026-05-26 主题 execution-plan.md |
| Phase 4: 独立信令服务器二进制 | 后续版本 | 2026-05-26 主题 execution-plan.md |
| Phase 4: 安装包 + 签名 + 用户文档 | 后续版本 | 2026-05-26 主题 execution-plan.md |
| 真正 WebRTC P2P 替换 WS 中继 | 未来 | 09-webrtc-implementation.md |

---

## 🗑️ 已放弃事项（仅记录）

| 事项 | 放弃原因 | 放弃时间 | 来源 |
|:---|:---|:---|:---|
| 单机双实例集成测试 | 已在 6.4 主题中确认丢弃，当前路线聚焦 LAN 自动发现与直连 | 2026-06-18 | 2026-06-04-[LOCKED]-single-machine-connection-testing |
