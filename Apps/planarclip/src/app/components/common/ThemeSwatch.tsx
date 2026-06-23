import type { ThemeColor } from "../../types";

type ThemeSwatchProps = {
  currentTheme: ThemeColor;
  selectedTheme: ThemeColor;
  isDark: boolean;
  size?: "md" | "lg";
  onChange: (theme: ThemeColor) => void;
};

export function ThemeSwatch({ currentTheme, selectedTheme, isDark, size = "md", onChange }: ThemeSwatchProps) {
  const color = isDark ? currentTheme.dark.primary : currentTheme.light.primary;
  const buttonSizeClassName = size === "lg" ? "h-9 w-9" : "h-6 w-6";
  const dotSizeClassName = size === "lg" ? "h-6 w-6" : "h-[18px] w-[18px]";

  return (
    <button
      aria-label={currentTheme.label}
      onClick={() => onChange(currentTheme)}
      className={`group flex ${buttonSizeClassName} items-center justify-center rounded-md`}
      title={currentTheme.label}
      type="button"
    >
      <span
        className={`${dotSizeClassName} rounded-full border-2 transition-transform group-hover:scale-105 ${selectedTheme.id === currentTheme.id ? "scale-105 border-foreground" : "border-transparent"}`}
        style={{ background: color, boxShadow: selectedTheme.id === currentTheme.id ? `0 0 8px ${color}` : "none" }}
      />
    </button>
  );
}
