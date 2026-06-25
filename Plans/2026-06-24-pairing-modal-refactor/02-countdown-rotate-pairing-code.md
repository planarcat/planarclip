# 第 2 轮：倒计时归零行为 — 轮换配对码并重置

> 时间: 2026-06-24

## 用户提问

倒计时归零只变更配对码，倒计时重置

## 背景与分析

### 对第 1 轮方案的修正

第 1 轮曾建议倒计时归零时提示「配对已超时」或进入失败态。用户明确要求：

- **归零时**：刷新本机配对码 + 倒计时从 60s 重新开始
- **不归零时**：不断开连接、不关闭弹窗、不弹出超时错误
- **≤10s**：进度条与数字仍保持红色闪烁（urgent），表示即将轮换，而非失败

### 当前后端限制

配对码目前由密钥指纹静态派生（`pairing_code_from_key_pair`），`get_pairing_code` 始终返回同一组数字。`direct.rs` 中虽有 `generate_pairing_code()`（随机 6 位），但仅用于测试。

握手侧 `responder_verify_code` 在 60s 无有效输入后会返回 `timeout` 并终止会话。因此 **仅前端改倒计时无法真正「变更配对码」**，必须后端参与。

### 行为矩阵（修正后）

| 事件 | 旧假设 | 修正后 |
|------|--------|--------|
| 倒计时归零 | 报错 / 超时 | 轮换配对码 + 重置 60s |
| 用户输错码 | 报错 | 仍报错（与轮换无关） |
| 用户点关闭 | 断开 | 不变 |
| 切换列表设备 | 重置流程 | 不变 + 重新生成该会话码 |
| 后端 TCP 真正断开 | 失败 | 不变 |

## 建议与回答

### 推荐实现：会话级动态配对码

**后端（Rust）**

1. `AppState` 增加：
   - `active_pairing_code: Arc<Mutex<Option<String>>>`
   - `pairing_code_issued_at: Arc<Mutex<Option<Instant>>>`

2. `get_pairing_code`：有活跃配对会话时返回 `active_pairing_code`；无会话时可返回指纹码或首次打开时生成会话码。

3. 新增 `rotate_pairing_code` 命令（或由握手循环内部触发）：
   - 调用 `generate_pairing_code()`
   - 更新 state + 向 UI 发送 `pairing-code-rotated { code, expires_at_ms }`

4. 修改 `responder_verify_code` 的 60s 超时逻辑：

   ```
   等待输入配对码
     ↓ 60s 无有效码
   生成新码 → 通知前端 → 继续等待（循环）
     ↓ 用户取消 / 对端断开
   才真正结束
   ```

   验证时比对**当前** `active_pairing_code`；旧码返回 `invalid_code`，但不因「窗口到期」而断连。

5. 入站陌生设备配对开始时生成首个会话码，与 UI 展示一致。

**前端**

1. `usePairingCountdown` 的 `onExpire`：
   ```typescript
   onExpire: () => {
     void callCommand("rotate_pairing_code");
     reset(60);
   }
   ```
   不归零时：不调 `disconnect`、不设 `stage = "error"`。

2. 订阅 `pairing-code-rotated` 事件，与本地倒计时双保险。

3. 视觉：正常 cyan；≤10s 红色闪烁；轮换瞬间数字可短暂过渡，进度条回满。

4. 文案：
   - 轮换提示：「验证码已更新，请让对方输入新的 6 位数字」
   - 输旧码：「配对码不正确，请查看对方屏幕上最新的验证码」

### 轮换作用范围（建议默认）

**仅在「等待验证」stage**（`awaiting_code` / `incoming_pairing`）下自动轮换；`idle` 或无活跃握手时不自动换码，避免用户尚未开始连接就看到码变化。

## 本轮要点

- 倒计时归零 = **轮换配对码 + 重置 60s**，不是失败/超时。
- 必须引入**会话级动态配对码**，前后端协同；静态指纹码不能满足产品语义。
- `responder_verify_code` 超时改为循环等待 + 轮换，而非一次性 fail。
- urgent（≤10s）仍是视觉预警，与归零轮换衔接，不表示错误。
