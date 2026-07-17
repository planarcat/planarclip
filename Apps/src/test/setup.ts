import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, vi } from "vitest";
import { cleanup } from "@testing-library/react";

import { resetTauriMocks } from "./tauri-mock";

// 每个用例前重置 Tauri IPC mock 状态，保证用例隔离。
beforeEach(() => {
  resetTauriMocks();
  if (typeof window !== "undefined" && window.localStorage) {
    window.localStorage.clear();
  }
});

// 每个用例后卸载 RTL 渲染的组件树。
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

// jsdom 未实现 matchMedia / ResizeObserver / IntersectionObserver，
// 组件中若引用需先在此处 stub，避免测试直接报错。
if (typeof window !== "undefined") {
  if (!window.matchMedia) {
    window.matchMedia = (query: string) =>
      ({
        matches: false,
        media: query,
        onchange: null,
        addListener: () => {},
        removeListener: () => {},
        addEventListener: () => {},
        removeEventListener: () => {},
        dispatchEvent: () => false,
      }) as unknown as MediaQueryList;
  }

  if (!(globalThis as { ResizeObserver?: unknown }).ResizeObserver) {
    class ResizeObserverStub {
      observe() {}
      unobserve() {}
      disconnect() {}
    }
    (globalThis as { ResizeObserver: unknown }).ResizeObserver =
      ResizeObserverStub;
  }

  // Vitest jsdom 环境下部分场景 window.localStorage 未被挂载，
  // 这里补一个最小内存 Storage 实现，保证 preview 相关工具函数可测。
  if (!window.localStorage) {
    const store = new Map<string, string>();
    const storage: Storage = {
      get length() {
        return store.size;
      },
      clear() {
        store.clear();
      },
      getItem(key: string) {
        return store.has(key) ? (store.get(key) as string) : null;
      },
      key(index: number) {
        return [...store.keys()][index] ?? null;
      },
      removeItem(key: string) {
        store.delete(key);
      },
      setItem(key: string, value: string) {
        store.set(key, String(value));
      },
    };
    Object.defineProperty(window, "localStorage", {
      value: storage,
      configurable: true,
    });
  }
}