# Vite+ 官方资料摘录

> 保存时间: 2026-06-18
> 用途: 作为本主题讨论的外部参考资料归档，避免仅依赖在线链接。

## 资料来源

- Why Vite+: https://viteplus.dev/guide/why
- Migrate to Vite+: https://viteplus.dev/guide/migrate
- Installing Dependencies: https://viteplus.dev/guide/install
- Announcing Vite+ Alpha: https://voidzero.dev/posts/announcing-vite-plus-alpha

## Why Vite+（关键摘录）

官方核心描述：

- Vite+ 将现代 Web 开发所需的工具整合进单一工具链，而不是让开发者自行拼装和维护自定义工具链。
- 它提供统一入口来管理运行时、依赖、开发服务器、代码质量检查、测试和构建。
- 实际包含：Vite、Rolldown、Vitest、Oxlint、Oxfmt、tsdown、Vite Task。
- 典型命令流：`vp dev`、`vp check`、`vp test`、`vp build`。

对本主题最相关的判断点：

- Vite+ 最大价值在“统一工具链”，不是单纯替换 Vite 本身。
- 如果项目当前还没有明显的工具链碎片化问题，则迁移收益可能有限。

## Migrate to Vite+（关键摘录）

官方迁移说明要点：

- 使用 `vp migrate` 作为迁移入口。
- 推荐先升级到 **Vite 8+** 与 **Vitest 4.1+**。
- 迁移过程会：
  - 更新项目依赖
  - 在必要时重写 import
  - 将工具配置合并到 `vite.config.ts`
  - 更新脚本到 Vite+ 命令面
  - 可选设置 hooks、agent、editor 配置
- 官方提醒：**大多数项目在跑完迁移命令后，仍需要人工微调**。
- 官方建议迁移后验证：
  - `vp install`
  - `vp check`
  - `vp test`
  - `vp build`

对本主题最相关的判断点：

- 当前仓库若要迁移，不能只改脚本；还需要人工核对 `vite.config.ts` 和 Tauri 侧联动。
- 当前仓库没有测试体系，因此官方推荐验证链里的 `vp test` 需要按实际情况落地。

## Installing Dependencies（关键摘录）

官方安装文档说明：

- `vp install` 会自动使用当前 workspace 的包管理器。
- Vite+ 按以下优先级推断包管理器：
  1. `packageManager` in `package.json`
  2. `devEngines.packageManager` in `package.json`
  3. `pnpm-workspace.yaml`
  4. `pnpm-lock.yaml`
  5. `yarn.lock` / `.yarnrc.yml`
  6. `package-lock.json`
  7. `bun.lock` / `bun.lockb`
  8. `.pnpmfile.cjs` / `pnpmfile.cjs`
  9. `bunfig.toml`
  10. `yarn.config.cjs`
- 如果项目未显式声明 `packageManager` 或 `devEngines.packageManager`，Vite+ 可能根据锁文件或配置文件推断并写入后续信息。

对本主题最相关的判断点：

- 当前仓库同时存在 npm / pnpm 痕迹，因此迁移前应先统一包管理器策略。
- 若不先收口，后续 `vp install` 的行为与团队预期可能不一致。

## Announcing Vite+ Alpha（关键摘录）

官方公告说明：

- Vite+ 已以 MIT 开源，定位为新的统一 Web 开发工具链入口。
- 它整合 Vite、Vitest、Oxlint、Oxfmt、Rolldown、tsdown 和 Vite Task。
- 常见命令包括：
  - `vp env`
  - `vp install`
  - `vp dev`
  - `vp check`
  - `vp test`
  - `vp build`
  - `vp run`
  - `vp pack`
  - `vp create`
- 官方强调：统一命令面可让人类与 AI 更容易共享一致工作流。
- 同时也明确表示：这是一个 **Alpha** 阶段产品，未来仍会频繁迭代。

对本主题最相关的判断点：

- 当前若要迁移，应把“统一工作流”视作主要收益，而不是把它当作性能升级按钮。
- 由于仍处于 Alpha，建议用“最小可用迁移 + 充分验证”的方式接入，而不是一次性做大规模重构。
