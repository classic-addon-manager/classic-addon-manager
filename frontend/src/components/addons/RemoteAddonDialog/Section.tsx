import type { ReactNode } from 'react'

import { cn } from '@/lib/utils.ts'

interface SectionProps {
  label: string
  children: ReactNode
  className?: string
}

export const Section = ({ label, children, className }: SectionProps) => (
  <section className={cn('border-t border-border/60 px-6 py-5', className)}>
    <h3 className="mb-3 text-[11px] font-medium uppercase tracking-wide text-primary/80">{label}</h3>
    {children}
  </section>
)
