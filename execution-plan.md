# GitNexus MCP Windows query 失效 — 待执行方案

> 生成时间: 2026-06-25
> 基于讨论: [01-mcp-health-check.md](01-mcp-health-check.md) | [02-recheck-after-analyze.md](02-recheck-after-analyze.md) | [03-windows-query-root-cause.md](03-windows-query-root-cause.md) | [04-solution-comparison-and-recommendation.md](04-solution-comparison-and-recommendation.md)

## 需求概述

在 Windows 开发环境下，GitNexus MCP 的 `query` 工具对 LaituUI（24432 embeddings）和 laitu-designer（17454 embeddings）返回空结果，并持续报 FTS 警告。需要将排查结论落盘，并按最可靠方案恢复完整的 GitNexus 代码智能搜索能力。

**影响范围：** 工作区两个项目（LaituUI、laitu-designer）共用的 GitNexus MCP 基础设施。

## 问题摘要


| 现象                     | 根因                                                       |
| ---------------------- | -------------------------------------------------------- |
| `query` 对 Laitu 大仓库返回空 | Windows 禁用 VECTOR + embedding 数 > 10000 时 exact-scan 被跳过 |
| FTS indexes missing 警告 | Windows MCP 进程故意不 LOAD fts（SIGSEGV 风险）                   |
| analyze 后问题仍在          | 平台限制，非索引损坏；re-analyze 无法修复                               |


**不受影响的能力：** `context`、`impact`、`detect_changes`、`cypher`、`list_repos`、资源读取。

## 技术决策


| 决策项                      | 选择                                           | 理由                                   | 来源轮次    |
| ------------------------ | -------------------------------------------- | ------------------------------------ | ------- |
| 主方案                      | **C — WSL/Linux 跑 GitNexus MCP**             | VECTOR + FTS 完整可用，可扩展，符合 GitNexus 设计 | 第 4 轮   |
| 辅助工具链                    | **B — context/impact/cypher/detect_changes** | Windows 上稳定，适合符号级深挖与变更影响分析           | 第 3、4 轮 |
| exact-scan 上限 workaround | **不采用（方案 A）**                                | 无 FTS、有上限、性能差、不可持续                   | 第 4 轮   |
| 等上游修复                    | **不采用（方案 D）**                                | 时间不可控                                | 第 4 轮   |
| 当前 Windows 索引            | 保留                                           | 图工具仍可用；WSL 侧重建索引                     | 第 3 轮   |


## 架构设计

### 目标态

```
Cursor (Windows)
    └── MCP: gitnexus (WSL/Linux 进程)
            ├── LadybugDB lbug（VECTOR 索引 ✅）
            ├── FTS 扩展（BM25 ✅）
            └── 索引仓库
                ├── LaituUI
                └── laitu-designer
```

### 路径策略

- WSL 内通过 `/mnt/c/Users/Administrator/Documents/Workspace/Laitu/...` 访问 Windows 工作区
- 在 WSL 内对两个前端仓库根目录分别执行 `gitnexus analyze --embeddings`
- Cursor `.cursor/mcp.json` 改为调用 WSL 内的 `gitnexus mcp`

## 实现步骤（含可执行命令）

以下命令按本机路径编写。PowerShell 在 **Windows** 执行；`bash` 块在 **WSL Ubuntu** 内执行。

---

### Phase 1: WSL 环境准备

#### 1.1 安装 WSL2 + Ubuntu（Windows 管理员 PowerShell）

```powershell
wsl --install -d Ubuntu
```

安装完成后**重启**。首次启动 Ubuntu 时创建 Linux 用户名和密码。

#### 1.2 验证 WSL2

```powershell
wsl -l -v
```

确认 Ubuntu 行 `VERSION` 为 `2`。若为 `1`：

```powershell
wsl --set-version Ubuntu 2
```

#### 1.3（可选）限制 WSL 内存上限，避免吃满宿主机

在 Windows 用户目录创建或编辑 `%UserProfile%\.wslconfig`：

```ini
[wsl2]
memory=16GB
processors=8
swap=8GB
```

保存后执行：

```powershell
wsl --shutdown
```

再重新 `wsl` 进入。

#### 1.4 安装 Node.js 22 + GitNexus（WSL 内）

```bash
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt-get install -y nodejs build-essential git
node --version
npm --version
```

国内网络建议先写入 `~/.bashrc`：

```bash
echo 'export HF_ENDPOINT=https://hf-mirror.com' >> ~/.bashrc
source ~/.bashrc
```

安装 GitNexus（与 Windows 同版本）：

```bash
sudo npm install -g gitnexus@1.6.5
gitnexus --version
which gitnexus
```

记下 `which gitnexus` 输出（常见为 `/usr/bin/gitnexus`），后面 MCP 配置可能用到。

#### 1.5 验收 Phase 1

```bash
gitnexus doctor
```

期望：`VECTOR index: available`、`Semantic mode: vector-index`（此时尚未 analyze 仓库，仅验证 Linux 运行时能力）。

---

### Phase 2: 索引重建（WSL 侧）

> **注意：** analyze 期间不要在 Cursor 里调用 GitNexus MCP `query`，避免 `LadybugDB unavailable` 文件锁。

#### 2.1 索引 LaituUI

```bash
cd /mnt/c/Users/Administrator/Documents/Workspace/Laitu/LaituUI/Apps/frontend/LaituUI
gitnexus analyze --embeddings --force
```

#### 2.2 索引 laitu-designer

```bash
cd /mnt/c/Users/Administrator/Documents/Workspace/Laitu/laitu-designer/Apps/frontend/laitu-designer
gitnexus analyze --embeddings --force
```

#### 2.3 检查 analyze 日志

成功标志——**不应出现**：

- `FTS extension unavailable`
- `Semantic embeddings were generated without a VECTOR index`

应出现类似：

- `Repository indexed successfully`
- 节点/边/flows 统计

#### 2.4 验收 Phase 2

```bash
cd /mnt/c/Users/Administrator/Documents/Workspace/Laitu/LaituUI/Apps/frontend/LaituUI
gitnexus status
gitnexus doctor
```

```bash
cd /mnt/c/Users/Administrator/Documents/Workspace/Laitu/laitu-designer/Apps/frontend/laitu-designer
gitnexus status
```

```bash
gitnexus list
```

确认列表含 `LaituUI`、`laitu-designer`，`indexedAt` 为本次 WSL analyze 时间。

#### 2.5（可选）Windows Defender 排除，减少 analyze 时扫盘卡顿

Windows **管理员 PowerShell**：

```powershell
Add-MpPreference -ExclusionPath "C:\Users\Administrator\Documents\Workspace\Laitu"
```

---

### Phase 3: Cursor MCP 配置

#### 3.1 编辑 `C:\Users\Administrator\.cursor\mcp.json`

将 `gitnexus` 段改为（保留 `figma-mcp-go` 不变）：

```json
{
  "mcpServers": {
    "gitnexus": {
      "command": "wsl",
      "args": [
        "-e", "bash", "-lc",
        "export HF_ENDPOINT=https://hf-mirror.com && gitnexus mcp"
      ]
    },
    "figma-mcp-go": {
      "command": "npx",
      "args": [
        "-y",
        "@vkhanhqui/figma-mcp-go"
      ]
    }
  }
}
```

若 `bash -lc` 找不到 `gitnexus`，改用绝对路径：

```json
"args": [
  "-e", "bash", "-lc",
  "export HF_ENDPOINT=https://hf-mirror.com && /usr/bin/gitnexus mcp"
]
```

#### 3.2 在 WSL 内预检 MCP 能否启动

```bash
gitnexus mcp
```

进程挂起、无报错即正常。`Ctrl+C` 退出。

#### 3.3 重启 Cursor MCP

Cursor：**Developer: Reload Window**，或在 MCP 设置面板重启 `gitnexus` 服务。

---

### Phase 4: 验证

在 Cursor 对话中让 AI 调用 GitNexus MCP（或自行通过 MCP 工具）：


| 步骤  | 操作                                                                    | 期望                                   |
| --- | --------------------------------------------------------------------- | ------------------------------------ |
| 4.1 | `list_repos`                                                          | 列出 LaituUI、laitu-designer、planarclip |
| 4.2 | `query({ query: "router", repo: "LaituUI", limit: 3 })`               | `processes` 非空                       |
| 4.3 | `query({ query: "ProductEditor", repo: "laitu-designer", limit: 3 })` | 非空                                   |
| 4.4 | 检查 query 响应                                                           | **无** `FTS indexes missing`          |
| 4.5 | `context({ name: "router", repo: "LaituUI" })`                        | 返回候选符号                               |


WSL 内辅助确认 registry：

```bash
cat ~/.gitnexus/registry.json
```

路径应为 `/mnt/c/Users/Administrator/...` 形式。

---

### Phase 5: 日常维护

#### 5.1 代码有较大变更后（WSL 内，两仓库各执行）

```bash
cd /mnt/c/Users/Administrator/Documents/Workspace/Laitu/LaituUI/Apps/frontend/LaituUI
gitnexus analyze --embeddings
```

```bash
cd /mnt/c/Users/Administrator/Documents/Workspace/Laitu/laitu-designer/Apps/frontend/laitu-designer
gitnexus analyze --embeddings
```

#### 5.2 检查索引是否过期

```bash
cd /mnt/c/Users/Administrator/Documents/Workspace/Laitu/LaituUI/Apps/frontend/LaituUI
gitnexus status
```

输出 `Status: ✅ up-to-date` 则无需重建；若落后 HEAD，再跑 `gitnexus analyze`。

#### 5.3 索引异常时全量重建

```bash
cd /mnt/c/Users/Administrator/Documents/Workspace/Laitu/LaituUI/Apps/frontend/LaituUI
gitnexus clean --force
gitnexus analyze --embeddings --force
```

laitu-designer 同理。

#### 5.4 日常开发（无需 WSL）

改代码、看影响范围时，可继续直接用 MCP：`context`、`impact`、`detect_changes`、`cypher`（索引在 WSL 侧 analyze 后即可）。

---

### 原 checklist（对照用）

#### Phase 1: WSL 环境准备

- 1.1～1.5 完成

#### Phase 2: 索引重建（WSL 侧）

- 2.1～2.4 完成

#### Phase 3: Cursor MCP 配置

- 3.1～3.3 完成

#### Phase 4: 验证

- 4.1～4.5 通过

#### Phase 5: 日常维护约定

- 已知 5.1～5.4 命令

## 关键依赖


| 依赖                        | 说明                                 |
| ------------------------- | ---------------------------------- |
| WSL2                      | 提供 Linux 运行时，绕过 Windows SIGSEGV 限制 |
| GitNexus 1.6.5            | 当前已安装版本                            |
| LadybugDB VECTOR + FTS 扩展 | 仅在 Linux/WSL 正常加载                  |
| Node.js v22               | 与当前 Windows 环境一致                   |
| Cursor MCP                | `.cursor/mcp.json` 配置              |


## 风险与注意事项

1. **双索引并存**：Windows 与 WSL 各有一套 `.gitnexus/`，需明确以 WSL 索引为 MCP 数据源，避免混淆。
2. **路径大小写**：WSL 访问 `/mnt/c/...` 时注意 Git 与文件路径一致性。
3. **analyze 耗时**：大仓库首次 `--embeddings --force` 可能需数十分钟（Windows 侧实测 LaituUI ~59s 增量、首次 ~50000s 含 embedding）。
4. **DB 锁**：analyze 进行中 MCP query 可能短暂报 LadybugDB unavailable，analyze 结束后重试即可。

## 参考讨论

- [第 1 轮：MCP 健康检查](01-mcp-health-check.md)
- [第 2 轮：analyze 后复检](02-recheck-after-analyze.md)
- [第 3 轮：Windows 根因排查](03-windows-query-root-cause.md)
- [第 4 轮：方案选型结论](04-solution-comparison-and-recommendation.md)
- [第 5 轮：方案 C 操作指南](05-wsl-implementation-guide.md)
- [第 6 轮：WSL/Docker/性能/风险](06-wsl-docker-performance-risks.md)

