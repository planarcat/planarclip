# Debug Session: lan-discovery
- **Status**: [OPEN]
- **Issue**: 两台设备都在线，但局域网设备列表仍然互相发现不到对方。
- **Debug Server**: http://198.18.0.1:7777/event
- **Log File**: .dbg/trae-debug-log-lan-discovery.ndjson

## Reproduction Steps
1. 在两台设备上启动桌面应用。
2. 确认两边都进入设备页并等待自动发现。
3. 观察设备列表仍为空，无法发现对方。

## Hypotheses & Verification
| ID | Hypothesis | Likelihood | Effort | Evidence |
|----|------------|------------|--------|----------|
| A | mDNS 启动成功，但实际上没有收到任何浏览事件 | High | Low | Pending |
| B | mDNS 收到了服务解析事件，但解析出的地址或 peer_id 被过滤掉了 | High | Low | Pending |
| C | 应用选错了本机网卡 IP，广播到了错误网段 | Medium | Medium | Pending |
| D | 前端没拿到后端事件，设备列表更新链路断了 | Medium | Low | Pending |
| E | 网络环境本身屏蔽了 mDNS 组播，导致双方都无发现事件 | Medium | Medium | Pending |

## Log Evidence
Pending

## Verification Conclusion
Pending
