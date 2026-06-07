import { Skeleton } from '@/components/ui/skeleton'

import type { AddonViewMode } from './types'

interface AddonSkeletonProps {
  variant?: AddonViewMode
}

export const AddonSkeleton = ({ variant = 'list' }: AddonSkeletonProps) => {
  if (variant === 'grid') {
    return (
      <div className="relative flex flex-col items-center gap-2 rounded-xl border border-border/50 bg-muted/30 px-3 py-3">
        <Skeleton className="h-14 w-14 rounded-xl" />
        <Skeleton className="h-3.5 w-3/4" />
        <Skeleton className="h-3 w-1/2" />
        <div className="flex gap-2">
          <Skeleton className="h-3 w-10" />
          <Skeleton className="h-3 w-10" />
        </div>
        <div className="flex gap-1">
          <Skeleton className="h-4 w-12 rounded-md" />
          <Skeleton className="h-4 w-12 rounded-md" />
        </div>
      </div>
    )
  }

  return (
    <div className="grid grid-cols-10 items-center justify-between bg-muted/50 aspect-video h-12 w-full rounded-lg">
      <div className="flex gap-3 col-span-4 items-center">
        <Skeleton className="ml-2 h-[38px] w-[38px]" />
        <Skeleton className="h-[20px] w-[150px]" />
      </div>
      <Skeleton className="col-span-2 h-[20px] w-[100px]" />
      <Skeleton className="col-span-2 h-[20px] w-[100px]" />
      <Skeleton className="col-span-1 h-[20px] w-[100px] ml-6" />
    </div>
  )
}
