import { Skeleton } from '@/components/ui/skeleton'

import { listRowGridClass } from './RemoteAddon/listLayout'
import type { AddonViewMode } from './types'

interface AddonSkeletonProps {
  variant?: AddonViewMode
}

export const AddonSkeleton = ({ variant = 'list' }: AddonSkeletonProps) => {
  if (variant === 'grid') {
    return (
      <div className="flex flex-col rounded-xl border border-border/60 bg-muted/20">
        <div className="flex items-center gap-2.5 px-3 pt-3">
          <Skeleton className="h-10 w-10 shrink-0 rounded-lg" />
          <div className="flex min-w-0 flex-1 flex-col gap-1.5">
            <Skeleton className="h-3.5 w-3/4" />
            <Skeleton className="h-3 w-1/2" />
          </div>
        </div>
        <div className="mt-2.5 flex min-h-8 flex-col justify-center gap-1.5 px-3">
          <Skeleton className="h-2.5 w-full" />
          <Skeleton className="h-2.5 w-2/3" />
        </div>
        <div className="mb-2.5 mt-2 flex h-5 items-center gap-1.5 px-3">
          <Skeleton className="h-4 w-12 rounded-full" />
          <Skeleton className="h-4 w-12 rounded-full" />
        </div>
        <div className="flex h-10 items-center gap-3 border-t border-border/60 px-3">
          <Skeleton className="h-3 w-12" />
          <Skeleton className="h-3 w-9" />
        </div>
      </div>
    )
  }

  return (
    <div className={`${listRowGridClass} w-full px-3 py-2.5`}>
      <Skeleton className="h-10 w-10 rounded-lg" />
      <div className="flex flex-col gap-1.5">
        <Skeleton className="h-4 w-40" />
        <Skeleton className="h-3 w-64 max-w-full" />
      </div>
      <Skeleton className="hidden h-3 w-28 md:block" />
      <Skeleton className="h-3 w-12 justify-self-end" />
      <Skeleton className="h-3 w-9 justify-self-end" />
      <span />
    </div>
  )
}
