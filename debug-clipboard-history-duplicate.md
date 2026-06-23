# Debug Session: clipboard-history-duplicate
- **Status**: [OPEN]
- **Issue**: 接收到其他设备发来的内容后，被同时识别为本机新复制内容并写入剪贴板历史，导致历史中出现重复项。
- **Debug Server**: http://127.0.0.1:7777/event
- **Log File**: .dbg/trae-debug-log-clipboard-history-duplicate.ndjson

## Reproduction Steps
1. 启动两台设备并建立连接。
2. 在设备 A 复制一段文本。
3. 观察设备 B 的剪贴板历史列表。
4. 问题表现为设备 B 同时出现“来自其他设备”和“来自我的设备”的重复记录。

## Hypotheses & Verification
| ID | Hypothesis | Likelihood | Effort | Evidence |
|----|------------|------------|--------|----------|
| A | 远端消息写入本地系统剪贴板后，被本地剪贴板监听器再次当作用户复制事件广播并入历史 | High | Med | Pending |
| B | 前端历史列表同时消费了“远端同步事件”和“本地监听事件”，但缺少内容去重或来源判定 | High | Med | Pending |
| C | 后端在接收远端内容时主动发出两类前端事件，导致 UI 追加两条记录 | Med | Med | Pending |
| D | 同一条远端内容经过连接层重放或重试，前端收到两次完全相同的同步消息 | Low | Med | Pending |

## Log Evidence
- 已确认 A：远端消息先进入连接层，然后被写入本机剪贴板，再被本机监听器二次识别成本地复制事件。
- 关键证据链（同一 hash）：
  - `received remote clipboard from direct connection` → 远端到达
  - `write_clipboard started` / `write_clipboard finished` → 写入本机系统剪贴板
  - `broadcasted remote clipboard event from direct connection` → 生成一条 `received`
  - 随后 `monitor emitted local clipboard event` → 又生成一条 `sent`
- 另外观察到瞬时空剪贴板读取会把监控状态打断，随后同一内容再次被识别为新本地内容，放大重复概率。
- B/C 被部分确认：前端本身只消费统一历史流，但后端历史汇总确实先后收到了 `received` 与 `sent` 两条事件，所以 UI 只是忠实显示了错误顺序。
- D 被否定：日志未显示连接层重复下发同一远端消息；重复来源发生在本机监听回路。

## Verification Conclusion
- 已实施修复，等待 post-fix 复现验证：
  1. 远端写入后，对同 hash 的本地回读在短窗口内抑制，不再生成 `sent`。
  2. 监控不再把临时空剪贴板读取当成新状态，避免 `last_hash` 被重置。
  3. 历史合并增加兜底：短时间内相同内容的对向重复项只保留远端 `received`。
