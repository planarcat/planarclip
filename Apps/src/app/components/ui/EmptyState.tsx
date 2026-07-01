import type { ReactNode } from "react";

type EmptyStateProps = {
  children: ReactNode;
  className?: string;
};

export function EmptyState({ children, className = "" }: EmptyStateProps) {
  return (
    <div
      className={`rounded-xl border border-dashed border-border px-3 py-10 text-center text-sm font-medium text-muted-foreground ${className}`}
    >
      {children}
    </div>
  );
}
