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
      onClick={() => onChange(currentTheme)}
      className="group flex flex-col items-center gap-1.5"
      title={currentTheme.label}
      type="button"
    >
      <span
        className={`h-6 w-6 rounded-full border-2 transition-transform group-hover:scale-110 ${selectedTheme.id === currentTheme.id ? "scale-110 border-foreground" : "border-transparent"}`}
        style={{ background: color, boxShadow: selectedTheme.id === currentTheme.id ? `0 0 8px ${color}` : "none" }}
      />
      <span className={`font-mono text-[10px] ${selectedTheme.id === currentTheme.id ? "text-primary" : "text-muted-foreground"}`}>
        {currentTheme.label}
      </span>
    </button>
  );
}
