import { useEffect, useRef } from "react";

const ENGAGED_HOLD_MS = 800;
const FADE_OUT_MS = 520;

export function useScrollbarReveal(root: HTMLElement | null) {
  const holdTimerRef = useRef<number>();
  const leaveTimerRef = useRef<number>();
  const pointerInsideRef = useRef(false);

  useEffect(() => {
    if (!root) {
      return;
    }

    const engage = () => {
      window.clearTimeout(leaveTimerRef.current);
      root.classList.remove("app-scrollbar-leaving");
      root.classList.add("app-scrollbar-engaged");
    };

    const startLeaving = () => {
      root.classList.remove("app-scrollbar-engaged");
      root.classList.add("app-scrollbar-leaving");
      window.clearTimeout(leaveTimerRef.current);
      leaveTimerRef.current = window.setTimeout(() => {
        root.classList.remove("app-scrollbar-leaving");
      }, FADE_OUT_MS);
    };

    const scheduleDisengage = () => {
      window.clearTimeout(holdTimerRef.current);
      holdTimerRef.current = window.setTimeout(() => {
        if (!pointerInsideRef.current) {
          startLeaving();
        }
      }, ENGAGED_HOLD_MS);
    };

    const onPointerEnter = () => {
      pointerInsideRef.current = true;
      window.clearTimeout(holdTimerRef.current);
      engage();
    };

    const onPointerLeave = () => {
      pointerInsideRef.current = false;
      window.clearTimeout(holdTimerRef.current);
      startLeaving();
    };

    const onScroll = () => {
      engage();
      scheduleDisengage();
    };

    root.addEventListener("pointerenter", onPointerEnter);
    root.addEventListener("pointerleave", onPointerLeave);
    root.addEventListener("scroll", onScroll, { passive: true });

    return () => {
      root.removeEventListener("pointerenter", onPointerEnter);
      root.removeEventListener("pointerleave", onPointerLeave);
      root.removeEventListener("scroll", onScroll);
      window.clearTimeout(holdTimerRef.current);
      window.clearTimeout(leaveTimerRef.current);
      root.classList.remove("app-scrollbar-engaged", "app-scrollbar-leaving");
      pointerInsideRef.current = false;
    };
  }, [root]);
}
