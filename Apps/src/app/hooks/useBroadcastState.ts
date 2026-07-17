import { invoke, isTauri } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { BroadcastState } from "../types";

/**
 * Tracks the backend broadcast state (mDNS + TCP listener). The backend pushes
 * `broadcast-state-changed` events on port conflicts / changes; we also pull the
 * initial state on mount.
 */
export function useBroadcastState(tauriAvailable: boolean): BroadcastState | undefined {
  const [state, setState] = useState<BroadcastState | undefined>(undefined);

  useEffect(() => {
    if (!tauriAvailable || !isTauri()) {
      return;
    }

    let disposed = false;
    let unlisten: UnlistenFn | undefined;

    void invoke<BroadcastState>("get_broadcast_state")
      .then((bs) => {
        if (!disposed) {
          setState(bs);
        }
      })
      .catch(() => {});

    void listen<BroadcastState>("broadcast-state-changed", (event) => {
      if (!disposed) {
        setState(event.payload);
      }
    }).then((un) => {
      if (disposed) {
        un();
      } else {
        unlisten = un;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [tauriAvailable]);

  return state;
}

/** Apply a new TCP listen port. Returns the resulting broadcast state. */
export async function setTcpPort(port: number): Promise<BroadcastState | undefined> {
  if (!isTauri()) {
    return undefined;
  }
  try {
    return await invoke<BroadcastState>("set_tcp_port", { port });
  } catch {
    return undefined;
  }
}
