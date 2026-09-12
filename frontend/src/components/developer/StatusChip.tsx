import { CircleCheck, CircleDashed, CircleX, Clock, type LucideIcon } from 'lucide-react'

import type { ReviewHistoryTone } from '@/components/developer/ownedParse'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

/**
 * One tone per review outcome. Color, icon and word always agree, so the status
 * never depends on hue alone, and an unmapped backend status stays an outline
 * instead of borrowing a real outcome's color.
 */
const TONES: Record<ReviewHistoryTone, { className: string; icon: LucideIcon }> = {
  review: {
    className: 'border-primary/60 bg-primary/15 text-primary-foreground dark:text-primary',
    icon: Clock,
  },
  approved: {
    className: 'border-emerald-500/40 bg-emerald-500/12 text-emerald-700 dark:text-emerald-300',
    icon: CircleCheck,
  },
  rejected: {
    className: 'border-destructive/40 bg-destructive/10 text-destructive',
    icon: CircleX,
  },
  unknown: {
    // The outline *is* the signal here, so it is stronger than a filled chip's border.
    className: 'border-dashed border-muted-foreground/40 bg-transparent text-muted-foreground',
    icon: CircleDashed,
  },
}

export function StatusChip({ tone, label }: { tone: ReviewHistoryTone; label: string }) {
  const { className, icon: Icon } = TONES[tone]
  return (
    <Badge variant="outline" className={cn('rounded-full py-0.5', className)}>
      <Icon aria-hidden />
      {label}
    </Badge>
  )
}
