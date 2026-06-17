# 未执行 / 遗留事项

> 生成时间: 2026-05-27
> 来源主题: 2026-05-26-cross-platform-clipboard-sync
> 继承自: 无
> 说明: 本文档将传递给下一个主题继续使用

---

## 📋 继承事项的处理结果

> 无继承事项（本主题为首个执行主题）

---

## ⏳ 继续传递的遗留事项

> 以下事项将传递给下一个主题

### ❌ 执行失败

（无）

### ⚠️ 未开发完成 / 完成度不足

| 事项 | 当前状态 | 差距描述 | 后续建议 |
|:---|:---|:---|:---|
| 单机双实例集成测试 | 未测试 | 需要信令服务器运行才能端到端验证 | 先部署/启动信令服务器，然后按 test plan 验证 |

### 📅 被移出本次开发节奏

（无）

### 🔮 计划在未来版本加入

| 事项 | 计划版本/Phase | 参考讨论 |
|:---|:---|:---|
| Phase 2: SQLite 存储（clipboard_entries + settings + peers + key_store）| Phase 2 | execution-plan.md |
| Phase 2: Noise IK 应用层加密 + ChaCha20-Poly1305 传输 | Phase 2 | execution-plan.md |
| Phase 2: 图片同步（arboard::get_image, ≤5MB）| Phase 2 | execution-plan.md |
| Phase 2: CI: GitHub Actions Win x64 + macOS ARM64 | Phase 2 | execution-plan.md |
| Phase 3: 文件同步 + 大文件分块传输 | Phase 3 | execution-plan.md |
| Phase 3: N 对等点 WebRTC 动态连接管理 | Phase 3 | execution-plan.md |
| Phase 3: 断线自动重连 + TURN 回退 | Phase 3 | execution-plan.md |
| Phase 3: 离线队列 + 冲突解决 | Phase 3 | execution-plan.md |
| Phase 4: 完整 React 前端 | Phase 4 | execution-plan.md |
| Phase 4: mDNS LAN 自动发现 | Phase 4 | execution-plan.md |
| Phase 4: 原生通知 + 暗色主题 + 快捷键 | Phase 4 | execution-plan.md |
| Phase 4: 独立信令服务器二进制 | Phase 4 | execution-plan.md |
| Phase 4: 安装包 + 签名 + 用户文档 | Phase 4 | execution-plan.md |
| 真正 WebRTC P2P 替换 WS 中继 | 未来 | 09-webrtc-implementation.md |

---

## 🗑️ 已放弃事项（仅记录，不传递）

> 以下事项已明确放弃，不再出现在后续主题的 UNEXECUTED.md 中

（无）
