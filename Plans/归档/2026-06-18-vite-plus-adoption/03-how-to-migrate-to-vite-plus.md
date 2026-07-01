# 第 3 轮：如果要上 Vite+，这个项目该怎么改造？

> 时间: 2026-06-18 00:20

## 用户提问

如果要上，怎么改造

## 背景与分析

继续结合仓库现状与 Vite+ 官方迁移资料进行分析：

- 官方迁移入口是 `vp migrate`，推荐先升级到 Vite 8+ 再迁移。
- 官方建议迁移后验证 `vp install`、`vp check`、`vp test`、`vp build`。
- Vite+ 会根据 `packageManager`、`pnpm-workspace.yaml`、锁文件等信息推断包管理器。
- 当前仓库里同时存在 npm / pnpm 痕迹，这会影响 Vite+ 的包管理器检测与后续命令一致性。
- 当前项目没有 `test`、`lint`、`format` 配置，因此如果只换掉 `vite` 命令而不补 `check`，迁移收益会非常有限。
- Tauri 的 `beforeDevCommand` / `beforeBuildCommand` 仍然是前端命令的关键接入点，因此迁移不能只改 `package.json`，还需要同步修改 `src-tauri/tauri.conf.json`。

参考资料：

- https://viteplus.dev/guide/migrate
- https://viteplus.dev/guide/install
- https://voidzero.dev/posts/announcing-vite-plus-alpha

## 建议与回答

**修改目标**
- 如果决定上 Vite+，**推荐把它作为“前端工具链统一改造”来做**，而不是只做一次依赖替换。
- 这个项目的最佳姿势不是“盲升”，而是 **先收口包管理器 → 再迁移 Vite 命令面 → 再把 check / test / docs 补齐**。

**先说结论**
- **推荐方案：做“最小可用迁移 + 顺手补基础工程化”**。
- 不建议只把 `vite` 改成 `vite-plus` 就收工，那样收益太薄。

**建议改动点**
- `Apps/planarclip/package.json`：当前只定义了 `vite`/`tsc`/`tauri` 脚本，没有 `check`、`test`，也没有声明 `packageManager`，这是迁移的主入口。
- `Apps/planarclip/vite.config.ts`：当前还是标准 Vite 配置，迁移后这里会成为统一配置入口。
- `Apps/planarclip/src-tauri/tauri.conf.json`：Tauri 通过 `beforeDevCommand` / `beforeBuildCommand` 串前端命令，迁移后这里也要联动。
- `Apps/planarclip/pnpm-workspace.yaml` + 锁文件：仓库里同时有 npm / pnpm 痕迹，而 Vite+ 会按优先级自动识别包管理器，这里必须先收口。
- `README.md` 和 `CLAUDE.md`：命令文档需要同步，不然团队会出现“代码用 vp，文档还在喊 npm”的错位。

**推荐改法**
- **第 1 步：先统一包管理器**
  - 推荐二选一：**继续用 npm** 或 **正式切到 pnpm**。
  - 不管选哪边，都建议在 `Apps/planarclip/package.json` 里显式加 `packageManager`。
  - 原因是 Vite+ 会优先从 `packageManager`、`pnpm-workspace.yaml`、锁文件判断包管理器；迁移前不收口，后面 `vp install` 的行为会有歧义。

- **第 2 步：先把 Vite 升到官方建议基线**
  - 当前是 `vite: ^6`。
  - 官方迁移建议是：**先升级到 Vite 8+**，再执行迁移。
  - 建议顺序是：
    - 先把 `vite` 升到 8+
    - 再引入 `vite-plus`
    - 再跑迁移

- **第 3 步：把前端命令面改成 Vite+**
  - 当前脚本：
    - `dev: vite`
    - `build: tsc && vite build`
    - `preview: vite preview`
  - 迁移后建议变成：
    - `dev: vp dev`
    - `build: vp build`
    - `preview: vp preview`
    - `check: vp check`
  - 如果后面加测试，再补：
    - `test: vp test`

- **第 4 步：把 `vite.config.ts` 变成统一配置入口**
  - 当前配置中的 Tauri dev server 端口、HMR、`TAURI_DEV_HOST` 适配逻辑建议保留。
  - 迁移后重点不是重写逻辑，而是把它作为统一的前端工具配置入口。

- **第 5 步：同步改 Tauri 的前置命令**
  - 当前：
    - `beforeDevCommand: "npm run dev"`
    - `beforeBuildCommand: "npm run build"`
  - 迁移后建议改成：
    - `beforeDevCommand: "vp dev"`
    - `beforeBuildCommand: "vp build"`
  - 但要注意：Vite+ 统一的是前端工具链，不会替代 Cargo 或 Tauri 后端构建链。

- **第 6 步：顺手补上 `vp check` 的实际收益**
  - 当前没有 ESLint / Prettier / Vitest 配置，也没有测试文件。
  - 所以如果决定迁，**建议至少把 `vp check` 落地**，否则只是换了命令名，没有真正吃到统一工具链的价值。

- **第 7 步：文档一起改**
  - `CLAUDE.md`、`README.md` 中现有命令说明需要同步成 `vp install`、`vp dev`、`vp build` 等新入口。
  - README 当前还有 merge conflict，建议在迁移同一批里顺手解决。

**推荐的实施顺序**
- **Phase 1：清理基础**
  - 选定包管理器
  - 删除另一套锁文件与多余痕迹
  - 给 `package.json` 补 `packageManager`
- **Phase 2：迁移工具链**
  - 升 Vite 到 8+
  - 引入 Vite+
  - 跑 `vp migrate`
  - 检查脚本和 import 是否被正确改写
- **Phase 3：联动 Tauri**
  - 改 `tauri.conf.json` 的前置命令
  - 确认 `npx tauri dev` 还能正常拉起前端
- **Phase 4：补工程化收益**
  - 增加 `vp check`
  - 视情况补 `vp test`
  - 更新 README / CLAUDE
- **Phase 5：验证**
  - `vp install`
  - `vp check`
  - `vp build`
  - `npx tauri dev`
  - `npx tauri build`

**更适合当前项目的两种方案**
- **方案 A：最小迁移，推荐**
  - 目标：先把命令入口统一到 Vite+，不大改项目结构。
  - 重点修改：`package.json`、`vite.config.ts`、`src-tauri/tauri.conf.json`、文档。
  - 优点：改动小、风险低、适合当前单应用 Tauri 项目。
  - 缺点：Vite+ 的价值只吃到一部分。

- **方案 B：完整迁移**
  - 在方案 A 基础上，再补：
    - `vp check`
    - 测试体系
    - hooks / staged 规则
    - 未来 workspace 规范
  - 优点：工具链真正统一。
  - 缺点：对当前项目规模来说，改造投入更大。

**更推荐的具体选择**
- **现在如果要上，就上“方案 A + 补 check”**。
- 原因是当前前端不复杂，Rust/Tauri 才是主战场，没必要一开始把所有工程化配置一口气端上来。

**风险与注意事项**
- **Vite 版本门槛**：当前还是 Vite 6，按官方建议应先升到 8+。
- **包管理器识别风险**：当前 npm / pnpm 痕迹并存，若不先收口，`vp install` 可能走出不想要的那条路。
- **Rust 不会被统一**：Vite+ 不会取代 Cargo、Tauri 的后端构建链。
- **Alpha 成熟度**：Vite+ 目前仍在 Alpha 阶段，迁移后应做更扎实的验证。
- **README 冲突要清理**：这不是 Vite+ 本身的坑，但会影响迁移后团队理解和使用。

## 本轮要点

- 推荐采用“最小可用迁移 + 补 check”的方式接入 Vite+。
- 包管理器统一、Vite 升级到 8+、Tauri 前置命令联动，是迁移的关键前提。
- 当前项目不适合一上来就做重工程化的完整迁移。
- 即使上了 Vite+，Rust/Tauri 仍然是主构建链的一部分，验证环节不能省。
