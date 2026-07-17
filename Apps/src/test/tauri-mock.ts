import { vi } from "vitest";

// Tauri IPC 手写 mock：拦截 @tauri-apps/api 的 invoke / listen / emit，
// 让 utils/hooks/组件层测试无需真实 Tauri 运行时也能执行。
//
// 用法：
//   import { mockInvoke, emitTauriEvent } from "@/test/tauri-mock";
//   mockInvoke("get_settings", () => ({ theme: "dark" }));
//   emitTauriEvent("lan-devices-changed", payload);

type InvokeHandler = (args: unknown) => unknown | Promise<unknown>;
type EventCallback = (event: { payload: unknown }) => void;

const invokeHandlers = new Map<string, InvokeHandler>();
const eventListeners = new Map<string, Set<EventCallback>>();

/** 为指定 Tauri command 注册返回值/异步处理器。 */
export function mockInvoke(command: string, handler: InvokeHandler) {
  invokeHandlers.set(command, handler);
}

/** 主动触发一个 Tauri 事件，模拟 Rust 侧 emit。 */
export function emitTauriEvent(event: string, payload: unknown) {
  const listeners = eventListeners.get(event);
  if (!listeners) return;
  for (const cb of listeners) cb({ payload });
}

/** 用例间清理所有 mock 状态；由 setup.ts 的 beforeEach 调用。 */
export function resetTauriMocks() {
  invokeHandlers.clear();
  eventListeners.clear();
}

// vi.mock 是模块级 hoist，必须在 import Tauri API 之前生效。
// 这里通过 vi.hoisted + 引用共享 Map 的形式，让所有 mock 状态由本文件唯一持有。
vi.mock("@tauri-apps/api/core", () => ({
  invoke: async (command: string, args?: unknown) => {
    const handler = invokeHandlers.get(command);
    if (!handler) {
      throw new Error(
        `[tauri-mock] invoke("${command}") 未注册处理器；` +
          `请在用例中调用 mockInvoke("${command}", ...)`,
      );
    }
    return handler(args);
  },
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: async (event: string, cb: EventCallback) => {
    let set = eventListeners.get(event);
    if (!set) {
      set = new Set();
      eventListeners.set(event, set);
    }
    set.add(cb);
    return () => {
      set?.delete(cb);
    };
  },
  emit: async (event: string, payload: unknown) => {
    emitTauriEvent(event, payload);
  },
  once: async (event: string, cb: EventCallback) => {
    const wrapper: EventCallback = (e) => {
      cb(e);
      eventListeners.get(event)?.delete(wrapper);
    };
    let set = eventListeners.get(event);
    if (!set) {
      set = new Set();
      eventListeners.set(event, set);
    }
    set.add(wrapper);
    return () => {
      set?.delete(wrapper);
    };
  },
}));

vi.mock("@tauri-apps/plugin-shell", () => ({
  open: async () => undefined,
}));