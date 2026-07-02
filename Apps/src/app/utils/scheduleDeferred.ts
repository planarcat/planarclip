/** Runs work after first paint / when the browser is idle. */
export function scheduleDeferred(work: () => void) {
  if (typeof requestIdleCallback === "function") {
    requestIdleCallback(() => work(), { timeout: 2_000 });
    return;
  }
  window.setTimeout(work, 0);
}
