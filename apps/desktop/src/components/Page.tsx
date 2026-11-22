// A page on the sheet: an optional eyebrow, a display-face title, and the content.
import { cn } from "@/lib/utils";

export function Page({
  eyebrow,
  title,
  actions,
  children,
  className,
}: {
  eyebrow?: string;
  title: string;
  actions?: React.ReactNode;
  children?: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("mx-auto flex w-full max-w-6xl flex-col gap-8 px-10 py-10", className)}>
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          {eyebrow && <p className="mb-1 font-display text-lg italic text-terracotta-ink">{eyebrow}</p>}
          <h1 className="font-display text-5xl font-semibold leading-tight text-primary-ink">{title}</h1>
        </div>
        {actions && <div className="flex items-center gap-2">{actions}</div>}
      </header>
      {children}
    </div>
  );
}

/** A friendly "not yet" card for features of later milestones. */
export function EmptyState({ icon: Icon, title, body }: { icon: React.ComponentType<{ className?: string }>; title: string; body: string }) {
  return (
    <section className="flex flex-col items-center gap-3 rounded-xl border bg-card px-8 py-16 text-center" data-testid="empty-state">
      <span className="flex size-12 items-center justify-center rounded-xl bg-muted text-muted-foreground" aria-hidden>
        <Icon className="size-6" />
      </span>
      <h2 className="font-display text-2xl font-semibold">{title}</h2>
      <p className="max-w-md text-muted-foreground">{body}</p>
    </section>
  );
}
