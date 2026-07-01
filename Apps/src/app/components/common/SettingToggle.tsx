import { SURFACE_REVEAL_BG, SETTINGS_CONTROL_COLUMN } from "../../constants/surfaceReveal";

type SettingToggleProps = {
  checked: boolean;
  disabled?: boolean;
  label: string;
  onChange: (checked: boolean) => void;
};

export function SettingToggle({ checked, disabled = false, label, onChange }: SettingToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`relative h-7 w-12 shrink-0 rounded-full border ${SURFACE_REVEAL_BG} aria-busy:opacity-80 disabled:cursor-not-allowed disabled:opacity-50 ${
        checked
          ? "border-primary bg-primary hover:bg-[var(--button-primary-hover-bg)]"
          : "border-border bg-secondary hover:border-primary/40 hover:bg-muted"
      }`}
    >
      <span
        className={`app-surface-reveal-switch-thumb pointer-events-none absolute top-1/2 h-5 w-5 -translate-y-1/2 rounded-full bg-white shadow ring-1 ring-black/10 ${
          checked ? "left-[calc(100%-1.375rem)]" : "left-0.5"
        }`}
      />
    </button>
  );
}

export function SettingToggleControl(props: SettingToggleProps) {
  return (
    <div className={SETTINGS_CONTROL_COLUMN}>
      <SettingToggle {...props} />
    </div>
  );
}