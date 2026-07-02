import { useCallback, useEffect, useRef, useState } from "react";
import type { ColorScheme, ThemeColor } from "../types";
import {
  captureThemeTokens,
  clearThemeTokenInlineOverrides,
  resolveAppearanceTokens,
  runThemeTokenBlend,
  type ThemeTokenMap,
} from "../utils/themeTokenBlend";
import { applyColorScheme, applyThemeColor } from "../utils/settings";
import {
  THEME_TRANSITION_EXPAND_MS,
  THEME_TRANSITION_ICON_EXIT_MS,
  THEME_TRANSITION_ICON_MS,
  type ThemePickOrigin,
  isColorSchemeDark,
  prefersReducedMotion,
} from "../utils/themeTransition";

export type BackgroundTransitionState = {
  kind: "background";
  phase: "icon" | "expand" | "iconExit";
  targetDark: boolean;
};

export type ThemeColorTransitionState = {
  kind: "theme";
  phase: "expand";
  color: string;
  origin: ThemePickOrigin;
};

export type ThemeTransitionState = BackgroundTransitionState | ThemeColorTransitionState;

function shouldRunTokenBlend(state: ThemeTransitionState | null) {
  if (!state) {
    return false;
  }
  if (state.kind === "theme") {
    return true;
  }
  return state.phase === "expand";
}

export function useThemeTransition() {
  const [state, setState] = useState<ThemeTransitionState | null>(null);
  const [appearanceTransitionActive, setAppearanceTransitionActive] = useState(false);
  const busyRef = useRef(false);
  const timersRef = useRef<number[]>([]);
  const pendingCompleteRef = useRef<(() => void) | null>(null);
  const blendPlanRef = useRef<{ from: ThemeTokenMap; to: ThemeTokenMap } | null>(null);
  const pendingAppearanceRef = useRef<{ scheme: ColorScheme; theme: ThemeColor } | null>(null);
  const stopBlendRef = useRef<(() => void) | null>(null);
  const appearanceCommittedRef = useRef(false);

  const clearTimers = useCallback(() => {
    timersRef.current.forEach((id) => window.clearTimeout(id));
    timersRef.current = [];
  }, []);

  const cancelBlendAnimation = useCallback(() => {
    stopBlendRef.current?.();
    stopBlendRef.current = null;
    document.documentElement.classList.remove("theme-token-blending");
  }, []);

  const commitTargetAppearance = useCallback(() => {
    const pending = pendingAppearanceRef.current;
    if (!pending) {
      clearThemeTokenInlineOverrides();
      return;
    }
    applyColorScheme(pending.scheme);
    clearThemeTokenInlineOverrides();
    applyThemeColor(pending.theme);
    appearanceCommittedRef.current = true;
  }, []);

  const releaseInteractionLock = useCallback(() => {
    cancelBlendAnimation();
    if (!appearanceCommittedRef.current) {
      commitTargetAppearance();
    }
    setAppearanceTransitionActive(false);
  }, [cancelBlendAnimation, commitTargetAppearance]);

  useEffect(() => () => {
    clearTimers();
    cancelBlendAnimation();
    clearThemeTokenInlineOverrides();
  }, [cancelBlendAnimation, clearTimers]);

  const schedule = useCallback((fn: () => void, delayMs: number) => {
    timersRef.current.push(window.setTimeout(fn, delayMs));
  }, []);

  const finalizeTransition = useCallback(() => {
    cancelBlendAnimation();
    if (!appearanceCommittedRef.current) {
      commitTargetAppearance();
    }
    pendingCompleteRef.current?.();
    pendingCompleteRef.current = null;
    blendPlanRef.current = null;
    pendingAppearanceRef.current = null;
    appearanceCommittedRef.current = false;
    setState(null);
    busyRef.current = false;
    setAppearanceTransitionActive(false);
    clearTimers();
  }, [cancelBlendAnimation, clearTimers, commitTargetAppearance]);

  const handleRevealEnd = useCallback(() => {
    if (!busyRef.current) {
      return;
    }
    finalizeTransition();
  }, [finalizeTransition]);

  const handleBackgroundCircleEnd = useCallback(() => {
    if (!busyRef.current) {
      return;
    }
    releaseInteractionLock();
    setState((current) =>
      current?.kind === "background" ? { ...current, phase: "iconExit" } : current,
    );
  }, [releaseInteractionLock]);

  useEffect(() => {
    if (!shouldRunTokenBlend(state) || !blendPlanRef.current) {
      return;
    }

    const plan = blendPlanRef.current;
    const linearSchemeBlend = state?.kind === "background";
    document.documentElement.classList.add("theme-token-blending");
    stopBlendRef.current?.();
    stopBlendRef.current = runThemeTokenBlend({
      from: captureThemeTokens(),
      to: plan.to,
      durationMs: THEME_TRANSITION_EXPAND_MS,
      linearColorProgress: linearSchemeBlend,
    });

    return () => {
      stopBlendRef.current?.();
      stopBlendRef.current = null;
    };
  }, [state]);

  const playBackgroundTransition = useCallback(
    (targetScheme: ColorScheme, theme: ThemeColor, onComplete: () => void) => {
      if (busyRef.current) {
        return;
      }
      if (prefersReducedMotion()) {
        onComplete();
        return;
      }

      busyRef.current = true;
      appearanceCommittedRef.current = false;
      setAppearanceTransitionActive(true);
      pendingCompleteRef.current = onComplete;
      pendingAppearanceRef.current = { scheme: targetScheme, theme };
      blendPlanRef.current = {
        from: captureThemeTokens(),
        to: resolveAppearanceTokens(targetScheme, theme),
      };
      const targetDark = isColorSchemeDark(targetScheme);
      setState({ kind: "background", phase: "icon", targetDark });

      schedule(() => {
        setState((current) =>
          current?.kind === "background" ? { ...current, phase: "expand" } : current,
        );
      }, THEME_TRANSITION_ICON_MS);

      schedule(() => {
        if (busyRef.current) {
          finalizeTransition();
        }
      }, THEME_TRANSITION_ICON_MS + THEME_TRANSITION_EXPAND_MS + THEME_TRANSITION_ICON_EXIT_MS + 80);
    },
    [finalizeTransition, schedule],
  );

  const playThemeTransition = useCallback(
    (
      nextTheme: ThemeColor,
      colorScheme: ColorScheme,
      isDark: boolean,
      origin: ThemePickOrigin,
      onComplete: () => void,
    ) => {
      if (busyRef.current) {
        return;
      }
      if (prefersReducedMotion()) {
        onComplete();
        return;
      }

      busyRef.current = true;
      appearanceCommittedRef.current = false;
      setAppearanceTransitionActive(true);
      pendingCompleteRef.current = onComplete;
      pendingAppearanceRef.current = { scheme: colorScheme, theme: nextTheme };
      blendPlanRef.current = {
        from: captureThemeTokens(),
        to: resolveAppearanceTokens(colorScheme, nextTheme),
      };
      const color = isDark ? nextTheme.dark.primary : nextTheme.light.primary;
      setState({ kind: "theme", phase: "expand", color, origin });

      schedule(() => {
        if (busyRef.current) {
          finalizeTransition();
        }
      }, THEME_TRANSITION_EXPAND_MS + 80);
    },
    [finalizeTransition, schedule],
  );

  return {
    transitionState: state,
    playBackgroundTransition,
    playThemeTransition,
    handleRevealEnd,
    handleBackgroundCircleEnd,
    appearanceTransitionActive,
    isTransitioning: state != null,
  };
};
