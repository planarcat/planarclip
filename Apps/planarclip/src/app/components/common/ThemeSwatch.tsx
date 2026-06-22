import type { ThemeColor } from "../../types";

type ThemeSwatchProps = {
  currentTheme: ThemeColor;
  selectedTheme: ThemeColor;
  isDark: boolean;
  onChange: (theme: ThemeColor) => void;
};

export function ThemeSwatch({ currentTheme, selectedTheme, isDark, onChange }: ThemeSwatchProps) {
  const color = isDark ? currentTheme.dark.primary : currentTheme.light.primary;

  return (
    <button
      aria-label={currentTheme.label}
      onClick={() => onChange(currentTheme)}
      className="group flex h-6 w-6 items-center justify-center rounded-md"
      title={currentTheme.label}
      type="button"
    >
      <span
        className={`h-[18px] w-[18px] rounded-full border-2 transition-transform group-hover:scale-105 ${selectedTheme.id === currentTheme.id ? "scale-105 border-foreground" : "border-transparent"}`}
        style={{ background: color, boxShadow: selectedTheme.id === currentTheme.id ? `0 0 8px ${color}` : "none" }}
      />
    </button>
  );
}
