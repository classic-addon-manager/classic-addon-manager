import type { ReactNode } from 'react'

export const Section = ({
  title,
  description,
  children,
}: {
  title: string
  description?: string
  children: ReactNode
}) => (
  <section className="space-y-3">
    <div className="px-1">
      <h2 className="text-sm font-medium tracking-tight">{title}</h2>
      {description && <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>}
    </div>
    <div className="overflow-hidden rounded-xl border border-border/60 bg-card/40">{children}</div>
  </section>
)
