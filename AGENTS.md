<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **planarclip** (1149 symbols, 1736 relationships, 66 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/planarclip/context` | Codebase overview, check index freshness |
| `gitnexus://repo/planarclip/clusters` | All functional areas |
| `gitnexus://repo/planarclip/processes` | All execution flows |
| `gitnexus://repo/planarclip/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->

# 产品文案约定

## 用户侧 UI 文案

- 用户可见的界面文本、按钮文案、状态文案、成功提示、警告提示、错误提示，默认全部使用中文。
- 提示文案必须使用自然语言，优先描述用户当前遇到的问题、系统正在做的事，以及用户下一步可以怎么做。
- 禁止直接向用户暴露程序术语、底层异常、协议名、库报错、系统调用错误、英文错误原文，除非用户明确进入开发排障场景。
- 如果底层返回的是技术错误，面向用户展示时必须先转换成自然语言；必要时可附带简短建议，例如检查网络、确认对端已启动、稍后重试。
- 成功提示应简洁明确，例如“已连接到设备”“配对成功”“已完成同步”；不要使用生硬的工程术语。
- 警告与失败提示应说明原因和影响，例如“未能连接到对方设备，请确认对方应用已打开”；不要只显示“连接失败”或原始错误对象。
- 同一类状态提示保持口径一致，避免同一流程里中英混用、术语混乱或一个地方说“配对”另一个地方说“握手”。
- 如果确实需要保留技术细节用于调试，应将技术细节写入日志或开发者输出，而不是直接展示在用户界面中。

## 交互与按钮规范

- 设计按钮等交互元素时，优先使用纯图标样式，尤其是刷新、增加、删除这类简单操作。
- 能用图标准确表达的按钮默认不再附带文字，但必须提供清晰的悬浮提示或无障碍标签。
- 连接、断开、刷新等高频动作应优先放在列表项右侧或标题操作区，保持就近操作与视觉统一。
