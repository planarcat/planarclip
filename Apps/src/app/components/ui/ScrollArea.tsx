import { useState, type ComponentPropsWithoutRef } from "react";
import { useScrollbarReveal } from "../../hooks/useScrollbarReveal";

type ScrollAreaProps = ComponentPropsWithoutRef<"div"> & {
  as?: "div" | "aside";
};

export function ScrollArea({ as = "div", className = "", ...props }: ScrollAreaProps) {
  const [root, setRoot] = useState<HTMLElement | null>(null);
  useScrollbarReveal(root);

  const mergedClassName = `app-scrollbar ${className}`.trim();

  if (as === "aside") {
    const asideProps = props as ComponentPropsWithoutRef<"aside">;
    return <aside ref={setRoot} className={mergedClassName} {...asideProps} />;
  }

  return <div ref={setRoot} className={mergedClassName} {...props} />;
}
