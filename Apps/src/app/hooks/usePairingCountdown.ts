import { useEffect, useRef, useState } from "react";

export const PAIRING_COUNTDOWN_SECS = 60;
export const PAIRING_URGENT_THRESHOLD_SECS = 10;

type UsePairingCountdownOptions = {
  active: boolean;
  onExpire: () => void;
  durationSecs?: number;
};

export function usePairingCountdown({ active, onExpire, durationSecs = PAIRING_COUNTDOWN_SECS }: UsePairingCountdownOptions) {
  const [remainingSeconds, setRemainingSeconds] = useState(durationSecs);
  const onExpireRef = useRef(onExpire);

  useEffect(() => {
    onExpireRef.current = onExpire;
  }, [onExpire]);

  useEffect(() => {
    if (!active) {
      setRemainingSeconds(durationSecs);
      return;
    }

    setRemainingSeconds(durationSecs);
    const timer = window.setInterval(() => {
      setRemainingSeconds((prev) => {
        if (prev <= 1) {
          onExpireRef.current();
          return durationSecs;
        }
        return prev - 1;
      });
    }, 1000);

    return () => window.clearInterval(timer);
  }, [active, durationSecs]);

  return {
    remainingSeconds,
    progress: remainingSeconds / durationSecs,
    isUrgent: remainingSeconds <= PAIRING_URGENT_THRESHOLD_SECS,
  };
}
