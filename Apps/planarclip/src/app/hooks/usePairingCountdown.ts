import { useEffect, useRef, useState } from "react";

export const PAIRING_COUNTDOWN_SECS = 60;
export const PAIRING_URGENT_THRESHOLD_SECS = 10;

type UsePairingCountdownOptions = {
  active: boolean;
  onExpire: () => void;
};

export function usePairingCountdown({ active, onExpire }: UsePairingCountdownOptions) {
  const [remainingSeconds, setRemainingSeconds] = useState(PAIRING_COUNTDOWN_SECS);
  const onExpireRef = useRef(onExpire);

  useEffect(() => {
    onExpireRef.current = onExpire;
  }, [onExpire]);

  useEffect(() => {
    if (!active) {
      setRemainingSeconds(PAIRING_COUNTDOWN_SECS);
      return;
    }

    setRemainingSeconds(PAIRING_COUNTDOWN_SECS);
    const timer = window.setInterval(() => {
      setRemainingSeconds((prev) => {
        if (prev <= 1) {
          onExpireRef.current();
          return PAIRING_COUNTDOWN_SECS;
        }
        return prev - 1;
      });
    }, 1000);

    return () => window.clearInterval(timer);
  }, [active]);

  return {
    remainingSeconds,
    progress: remainingSeconds / PAIRING_COUNTDOWN_SECS,
    isUrgent: remainingSeconds <= PAIRING_URGENT_THRESHOLD_SECS,
  };
}
