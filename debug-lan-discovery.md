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
| A | mDNS 启动成功，但实际上没有收到任何浏览事件 | High | Low | Partially Rejected |
| B | mDNS 收到了服务解析事件，但解析出的地址或 peer_id 被过滤掉了 | High | Low | Rejected |
| C | 应用选错了本机网卡 IP，广播到了错误网段 | Medium | Medium | Confirmed |
| D | 前端没拿到后端事件，设备列表更新链路断了 | Medium | Low | Rejected |
| E | 网络环境本身屏蔽了 mDNS 组播，导致双方都无发现事件 | Medium | Medium | Inconclusive |

## Log Evidence
- line 1: 启动时 `selected_ip` 被选成 `fdfe:dcba:9876::1`，而接口列表里同时存在 `WLAN=192.168.0.28`，说明当前 IP 选择策略优先命中了非局域网私有 IPv4 地址。
- line 2-9: 浏览阶段能够解析到服务，但全部是本机自身服务，且被 `is_self=true` 正常过滤；没有看到来自对端的服务解析事件。
- line 1 与接口快照共同表明，发现注册很可能绑定在虚拟 / 非目标网段地址上，导致对端无法通过同一局域网正常发现本机。

## Verification Conclusion
已确认首个根因是 mDNS 选错本机地址，并已修成优先私有 IPv4。post-fix 日志显示本机现在使用 `192.168.0.28` 注册服务；结合用户反馈“对方发现了我，我没有发现对方”，说明本机发布链路已恢复，但对端发布链路仍未被本机观察到。当前更可能是另一台设备仍在旧版本 / 未重启到新构建，或另一台设备本身还有注册失败问题。
