import type { ThemeColor } from "../../types";
import type { ThemePickOrigin } from "../../utils/themeTransition";
import { SURFACE_REVEAL_BG } from "../../constants/surfaceReveal";

type ThemeSwatchProps = {
  currentTheme: ThemeColor;
  selectedTheme: ThemeColor;
  isDark: boolean;
  size?: "md" | "lg";
  onChange: (theme: ThemeColor, origin: ThemePickOrigin) => void;
};

export function ThemeSwatch({ currentTheme, selectedTheme, isDark, size = "md", onChange }: ThemeSwatchProps) {
  const color = isDark ? currentTheme.dark.primary : currentTheme.light.primary;
  const buttonSizeClassName = size === "lg" ? "h-9 w-9" : "h-6 w-6";
  const dotSizeClassName = size === "lg" ? "h-5 w-5" : "h-4 w-4";
  const isSelected = selectedTheme.id === currentTheme.id;

  return (
    <button
      aria-label={currentTheme.label}
      onClick={(event) => {
        const rect = event.currentTarget.getBoundingClientRect();
        onChange(currentTheme, {
          x: rect.left + rect.width / 2,
          y: rect.top + rect.height / 2,
        });
      }}
      className={`group flex ${buttonSizeClassName} items-center justify-center rounded-md ${SURFACE_REVEAL_BG} hover:bg-secondary/50`}
      title={currentTheme.label}
      type="button"
    >
      <span
        className={`${dotSizeClassName} box-border shrink-0 rounded-full border-2 transition-transform group-hover:scale-105 ${
          isSelected ? "border-foreground" : "border-transparent"
        }`}
        style={{ background: color, boxShadow: isSelected ? `0 0 8px ${color}` : "none" }}
      />
    </button>
  );
}
