import { Moon, Sun } from "lucide-react";
import type { ThemeTransitionState } from "../../hooks/useThemeTransition";
import {
  THEME_TRANSITION_CIRCLE_REVEAL_ANIMATION,
  THEME_TRANSITION_ICON_EXIT_ANIMATION,
  themeTransitionCoverScale,
  themeTransitionTopRightOrigin,
} from "../../utils/themeTransition";
import { OverlayPortal } from "../ui/OverlayPortal";

type ThemeTransitionOverlayProps = {
  state: ThemeTransitionState | null;
  onRevealEnd: () => void;
  onBackgroundCircleEnd: () => void;
};

const overlayRootClass = "pointer-events-none fixed inset-0 z-[100] overflow-visible";

const BACKGROUND_ICON_SIZE = 52;

type RevealCircleProps = {
  originX: number;
  originY: number;
  fill: string;
  glow: string;
  onRevealEnd: () => void;
};

function RevealCircle({ originX, originY, fill, glow, onRevealEnd }: RevealCircleProps) {
  const coverScale = themeTransitionCoverScale(originX, originY);

  return (
    <div
      className="fixed h-0 w-0 overflow-visible"
      style={{ left: originX, top: originY }}
    >
      <div
        className="theme-transition-circle"
        style={{
          backgroundColor: fill,
          boxShadow: glow,
          ["--theme-transition-cover-scale" as string]: coverScale,
        }}
        onAnimationEnd={(event) => {
          const name = event.animationName;
          if (name !== THEME_TRANSITION_CIRCLE_REVEAL_ANIMATION && !name.endsWith(THEME_TRANSITION_CIRCLE_REVEAL_ANIMATION)) {
            return;
          }
          onRevealEnd();
        }}
      />
    </div>
  );
}

function backgroundIconClass(phase: "icon" | "expand" | "iconExit") {
  if (phase === "icon") {
    return "theme-transition-icon-enter";
  }
  if (phase === "iconExit") {
    return "theme-transition-icon-exit";
  }
  return "theme-transition-icon-held";
}

type BackgroundSchemeIconProps = {
  targetDark: boolean;
  phase: "icon" | "expand" | "iconExit";
  onExitEnd: () => void;
};

function BackgroundSchemeIcon({ targetDark, phase, onExitEnd }: BackgroundSchemeIconProps) {
  return (
    <div
      className={`theme-transition-icon-layer absolute right-4 top-4 ${backgroundIconClass(phase)}`}
      onAnimationEnd={(event) => {
        if (phase !== "iconExit") {
          return;
        }
        const name = event.animationName;
        if (name !== THEME_TRANSITION_ICON_EXIT_ANIMATION && !name.endsWith(THEME_TRANSITION_ICON_EXIT_ANIMATION)) {
          return;
        }
        onExitEnd();
      }}
    >
      {targetDark ? (
        <Moon size={BACKGROUND_ICON_SIZE} className="text-slate-200 drop-shadow-lg" strokeWidth={1.75} />
      ) : (
        <Sun size={BACKGROUND_ICON_SIZE} className="text-amber-400 drop-shadow-lg" strokeWidth={1.75} />
      )}
    </div>
  );
}

export function ThemeTransitionOverlay({
  state,
  onRevealEnd,
  onBackgroundCircleEnd,
}: ThemeTransitionOverlayProps) {
  if (!state) {
    return null;
  }

  if (state.kind === "background") {
    const glow = state.targetDark
      ? "0 0 48px 12px color-mix(in oklab, var(--background) 65%, black)"
      : "0 0 48px 12px color-mix(in oklab, var(--background) 55%, white)";
    const origin = themeTransitionTopRightOrigin();
    const showIcon = state.phase === "icon" || state.phase === "expand" || state.phase === "iconExit";

    return (
      <OverlayPortal>
        <div className={overlayRootClass} aria-hidden role="presentation">
          {showIcon ? (
            <BackgroundSchemeIcon targetDark={state.targetDark} phase={state.phase} onExitEnd={onRevealEnd} />
          ) : null}
          {state.phase === "expand" ? (
            <RevealCircle
              originX={origin.x}
              originY={origin.y}
              fill="var(--background)"
              glow={glow}
              onRevealEnd={onBackgroundCircleEnd}
            />
          ) : null}
        </div>
      </OverlayPortal>
    );
  }

  const glow = `0 0 60px 16px color-mix(in srgb, ${state.color} 45%, transparent)`;

  return (
    <OverlayPortal>
      <div className={overlayRootClass} aria-hidden role="presentation">
        <RevealCircle
          originX={state.origin.x}
          originY={state.origin.y}
          fill={state.color}
          glow={glow}
          onRevealEnd={onRevealEnd}
        />
      </div>
    </OverlayPortal>
  );
}
