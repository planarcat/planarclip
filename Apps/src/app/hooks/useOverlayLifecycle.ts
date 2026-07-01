import { useEffect, useState } from "react";

const DEFAULT_EXIT_MS = 160;

export function useOverlayLifecycle(open: boolean, exitDurationMs = DEFAULT_EXIT_MS) {
  const [mounted, setMounted] = useState(open);
  const [exiting, setExiting] = useState(false);

  useEffect(() => {
    if (open) {
      setMounted(true);
      setExiting(false);
      return;
    }

    if (!mounted) {
      return;
    }

    setExiting(true);
    const timer = window.setTimeout(() => {
      setMounted(false);
      setExiting(false);
    }, exitDurationMs);

    return () => window.clearTimeout(timer);
  }, [open, mounted, exitDurationMs]);

  return { mounted, exiting };
}
