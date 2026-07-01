/** Background hover/focus timing shared with scrollbar thumb reveal (`surface-reveal.css`). */
export const SURFACE_REVEAL_BG = "app-surface-reveal-bg";

const fieldReveal =
  `${SURFACE_REVEAL_BG} border border-border bg-secondary hover:bg-card/60 focus:border-primary focus:bg-card focus:outline-none disabled:cursor-not-allowed disabled:opacity-60`;

/** Default text input (settings, forms). */
export const SURFACE_REVEAL_TEXT_FIELD = `rounded-lg px-3 py-2.5 text-sm font-medium text-foreground placeholder:text-muted-foreground/60 ${fieldReveal}`;

/** Compact text input (sidebar device name). */
export const SURFACE_REVEAL_TEXT_FIELD_SM = `rounded-md px-2.5 py-1.5 text-[13px] font-medium text-primary placeholder:text-primary/45 ${fieldReveal}`;

/** Native select. */
export const SURFACE_REVEAL_SELECT = `rounded-lg px-3 py-2 text-sm font-medium text-foreground ${fieldReveal}`;

/** Pairing code / mono input. */
export const SURFACE_REVEAL_CODE_FIELD = `rounded-lg border border-border bg-secondary px-3 py-2.5 text-center font-mono text-base tracking-[0.2em] text-foreground placeholder:text-muted-foreground/60 ${SURFACE_REVEAL_BG} hover:bg-card/60 focus:border-primary focus:bg-card focus:outline-none disabled:opacity-50`;

/** Sidebar nav item shell (icon column + label). */
export const SURFACE_REVEAL_NAV_ITEM = `${SURFACE_REVEAL_BG} grid w-full min-h-9 grid-cols-[15px_1fr] items-center gap-x-2.5 rounded-lg px-2.5 py-2 text-left text-sm font-medium`;

/** Settings row control column (toggles). */
export const SETTINGS_CONTROL_COLUMN = "flex w-12 shrink-0 items-center justify-end";
