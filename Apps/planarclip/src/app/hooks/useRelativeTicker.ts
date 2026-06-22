import { useEffect, useState } from "react";

export function useRelativeTicker() {
  const [, setTick] = useState(0);

  useEffect(() => {
    const timer = window.setInterval(() => setTick((value) => value + 1), 5_000);
    return () => window.clearInterval(timer);
  }, []);
}
