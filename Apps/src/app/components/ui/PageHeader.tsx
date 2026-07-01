import type { ReactNode } from "react";

type PageHeaderProps = {
  title: ReactNode;
  actions?: ReactNode;
  subtitle?: ReactNode;
};

export function PageHeader({ title, actions, subtitle }: PageHeaderProps) {
  return (
    <header className="flex shrink-0 items-center justify-between gap-4 border-b border-border px-6 py-4">
      <div className="min-w-0">
        <h1 className="text-base font-semibold text-foreground">{title}</h1>
        {subtitle ? <p className="mt-0.5 text-xs font-medium text-muted-foreground">{subtitle}</p> : null}
      </div>
      {actions ? <div className="flex shrink-0 items-center gap-2">{actions}</div> : null}
    </header>
  );
}
