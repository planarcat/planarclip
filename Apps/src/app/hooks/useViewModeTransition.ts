import { useEffect, useRef, useState } from "react";
import type { ViewMode } from "../types";

const VIEW_MODE_EXIT_MS = 160;

function prefersReducedMotion() {
  if (typeof window === "undefined") {
    return false;
  }
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * Delays swapping list/grid DOM until exit animation completes, then plays enter animation.
 */
export function useViewModeTransition(targetMode: ViewMode) {
  const [displayMode, setDisplayMode] = useState(targetMode);
  const [exiting, setExiting] = useState(false);
  const skipEnterAnimationRef = useRef(true);

  useEffect(() => {
    if (targetMode === displayMode) {
      return;
    }

    skipEnterAnimationRef.current = false;

    if (prefersReducedMotion()) {
      setDisplayMode(targetMode);
      setExiting(false);
      return;
    }

    setExiting(true);
    const timer = window.setTimeout(() => {
      setDisplayMode(targetMode);
      setExiting(false);
    }, VIEW_MODE_EXIT_MS);

    return () => window.clearTimeout(timer);
  }, [targetMode, displayMode]);

  const contentClass = exiting
    ? "clip-history-view-exit"
    : skipEnterAnimationRef.current
      ? ""
      : "clip-history-view-enter";

  return { displayMode, contentClass };
}
