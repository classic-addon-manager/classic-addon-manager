import { CalendarDaysIcon, DownloadIcon, HeartIcon, type LucideIcon, TagIcon } from 'lucide-react'
import type { ReactNode } from 'react'

import { Skeleton } from '@/components/ui/skeleton.tsx'
import { formatToLocalDate, formatToLocalTime } from '@/lib/utils.ts'
import type { AddonManifest, Release } from '@/lib/wails'

interface StatBarProps {
  manifest: AddonManifest
  release: Release | null
  isLoadingRelease: boolean
}

interface Stat {
  key: string
  icon: LucideIcon
  value: ReactNode
  label: string
  title?: string
}

export const StatBar = ({ manifest, release, isLoadingRelease }: StatBarProps) => {
  const releaseValue = (value: (loaded: Release) => ReactNode): ReactNode => {
    if (isLoadingRelease) return <Skeleton className="mx-auto h-4 w-20" />
    if (!release) return '—'
    return value(release)
  }

  const stats: (Stat | null)[] = [
    {
      key: 'downloads',
      icon: DownloadIcon,
      value: manifest.downloads.toLocaleString(),
      label: 'Downloads',
    },
    manifest.like_percentage === null || manifest.like_percentage === undefined
      ? null
      : {
          key: 'liked',
          icon: HeartIcon,
          value: `${manifest.like_percentage}%`,
          label: 'Liked',
        },
    {
      key: 'version',
      icon: TagIcon,
      value: releaseValue(loaded => loaded.tag_name),
      label: 'Version',
    },
    {
      key: 'updated',
      icon: CalendarDaysIcon,
      value: releaseValue(loaded => formatToLocalDate(loaded.published_at, 'short')),
      title: release ? formatToLocalTime(release.published_at) : undefined,
      label: 'Updated',
    },
  ]

  return (
    <div className="flex divide-x divide-border/60 border-y border-border/60">
      {stats
        .filter((stat): stat is Stat => stat !== null)
        .map(stat => (
          <div key={stat.key} className="min-w-0 flex-1 px-4 py-3 text-center" title={stat.title}>
            <stat.icon className="mx-auto mb-1.5 h-3.5 w-3.5 text-primary" />
            <div className="truncate text-sm font-semibold">{stat.value}</div>
            <div className="mt-0.5 text-xs text-muted-foreground">{stat.label}</div>
          </div>
        ))}
    </div>
  )
}
