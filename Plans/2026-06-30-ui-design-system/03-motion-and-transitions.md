# UI 规范 · 动画与转场

> 技术栈：Tailwind `transition-*`、`tw-animate-css`、少量自定义 `@keyframes`（如 `connection-attempt-marquee`）  
> 原则：**短、一致、可关闭**；桌面工具总时长多数 ≤ 250ms。

## 1. 动效令牌（建议写入 `index.css`）

```css
:root {
  --motion-fast: 120ms;
  --motion-normal: 180ms;
  --motion-slow: 240ms;
  --motion-enter: 220ms;
  --motion-exit: 160ms;
  --ease-out: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-in: cubic-bezier(0.4, 0, 1, 1);
  --ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);
}
```

Tailwind 用法示例（可在 `@layer utilities` 定义）：

```text
duration-fast   → var(--motion-fast)
duration-normal → var(--motion-normal)
ease-product    → var(--ease-out)
```

在组件中逐步把 `duration-1000`（配对倒计时条）保留为 **功能型长过渡**，交互类改用 token。

## 2. 动效分类与规范

### 2.1 微交互（默认开启）

| 场景 | 属性 | 时长 | 缓动 | 现网 |
|------|------|------|------|------|
| 颜色 hover | `transition-colors` | 150–180ms | ease-out | ✓ 广泛 |
| 开关滑块 | `transition-transform` + colors | 180ms | ease-out | SettingToggle ✓ |
| 主题色点 | `group-hover:scale-105` | 150ms | ease-out | ThemeSwatch ✓ |
| 主按钮 | `transition-opacity` | 150ms | — | ✓ |
| 禁用 | `opacity-40`/`50` | 无动画 | — | ✓ |

### 2.2 模态与遮罩（建议补齐）

**进入（mount）**

- 遮罩：`opacity 0 → 1`，220ms ease-out
- 面板：`opacity 0 → 1` + `scale(0.96 → 1)` + 可选 `translateY(8px → 0)`，220ms ease-out

**退出（unmount）**

- 遮罩 160ms ease-in；面板 scale(0.98) + opacity，160ms  
- 实现方式（按优先级）：
  1. **CSS + 状态类**：`data-state=open|closed` on 根节点，用 `tw-animate-css` 的 `animate-in` / `animate-out`（若已暴露）
  2. **轻量 hook**：`useExitAnimation` 延迟 unmount 160ms
  3. 不满足再考虑 `@radix-ui/react-dialog` 仅用于 focus trap + 动画

**点击遮罩关闭**：与退出动画共用，避免瞬间消失。

适用：`PairingModal`、`IncomingConnectionPrompt`。

### 2.3 页面导航（侧栏切换）

当前：`activeNav` 条件渲染，**无转场**。

推荐（低成本）：

```text
main 内层包一层 key={activeNav}
className="animate-in fade-in slide-in-from-right-2 duration-normal fill-mode-both"
```

- 剪贴板 ↔ 设备 ↔ 设置：**fade + 4px 位移**，220ms
- 不使用全屏 slide（像手机 App）

若 `tw-animate-css` 类名与 Tailwind v4 集成有差异，在 `index.css` 定义：

```css
@keyframes page-enter {
  from { opacity: 0; transform: translateX(6px); }
  to { opacity: 1; transform: translateX(0); }
}
.page-enter { animation: page-enter var(--motion-enter) var(--ease-out) both; }
```

### 2.4 列表与内容

| 场景 | 建议 |
|------|------|
| 剪贴板新条目 | 可选 `fade-in` 150ms；不 stagger（条目可能频繁） |
| 设备列表刷新 | 不闪烁整块列表；仅刷新图标 spin |
| 列表项删除 | 未来可加 height collapse；MVP 可不做 |
| 视图 list/grid 切换 | `cross-fade` 180ms on 容器 |

### 2.5 连接 / 配对 / 传输（业务动效）

| 组件 | 动效 | 说明 |
|------|------|------|
| `PairingModal` 倒计时条 | `transition-all duration-1000 ease-linear` | 保持；紧迫态 `animate-pulse` 仅 **isUrgent** |
| `ConnectionAttemptCard` | `connection-attempt-marquee` | 保持；`prefers-reduced-motion` 已放慢 |
| `Loader2` | `animate-spin` | 统一尺寸 14–15 |
| `StatusNotice` | 进入 slide-up + fade；退出 fade | 见下节 |
| `TransferProgressCard` | 进度条 `transition-[width]` 100ms linear | 与 `ProgressTrack` 对齐 |

**StatusNotice 目标**

- 进入：`translateY(12px)→0` + opacity，220ms
- 4s 后退出：opacity 160ms 再 unmount（在 onDismiss 前播完）
- 与右下角卡错开：notice 居中底部 z-80，传输卡 z-70

### 2.6 禁止 / 慎用

- 全页 `animate-bounce`、`blur-in`
- 长于 300ms 的入场（除 60s 配对进度条）
- 同时 pulse 多个元素（仅超时警告单点）
- 滚动容器内 `scale` hover（会裁切）

## 3. `prefers-reduced-motion`

在 `index.css` 扩展：

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
  .connection-attempt-flow {
    animation: none;
    opacity: 0.6;
  }
}
```

与现有 `connection-attempt-flow` 规则合并，避免重复 @media。

## 4. 共享工具（实施时）

建议新增（小文件，避免过度抽象）：

| 文件 | 职责 |
|------|------|
| `styles/motion.css` | keyframes、`.modal-enter`、`.page-enter`、`reduce-motion` |
| `hooks/useDelayedUnmount.ts` | 退出动画后真正卸载模态 |
| `components/common/ModalBackdrop.tsx` | 遮罩 + 面板壳 + 进入退出 class |

## 5. 验收标准（动效）

1. 配对开/关有可见但不拖沓的过渡  
2. 侧栏切页无「闪白」或布局跳动  
3. StatusNotice 出现/消失柔和  
4. 系统开启「减少动态效果」时，marquee 停止或极弱  
5. 全程无新增 >300ms 的交互动画（倒计时条除外）
