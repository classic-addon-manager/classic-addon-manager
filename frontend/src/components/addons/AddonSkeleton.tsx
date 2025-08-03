import { Skeleton } from '@/components/ui/skeleton'

export const AddonSkeleton = () => (
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
