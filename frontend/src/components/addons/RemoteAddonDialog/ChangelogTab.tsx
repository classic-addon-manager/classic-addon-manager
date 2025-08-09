import { CalendarDaysIcon, PackageIcon } from 'lucide-react'

import { RemoteAddonReadme } from '@/components/shared/RemoteAddonReadme.tsx'
import { formatToLocalTime } from '@/lib/utils.ts'
import type { Release } from '@/lib/wails'

interface ChangelogTabProps {
  release: Release | null
  changelog: string
}

export const ChangelogTab = ({ release, changelog }: ChangelogTabProps) => {
  if (release) {
    return (
      <>
        <div className="inline-flex items-center gap-1.5 text-xs text-muted-foreground mb-4">
          <CalendarDaysIcon className="w-3.5 h-3.5" />
          <span>Released on {formatToLocalTime(release.published_at)}</span>
        </div>
        <div className="border rounded-lg p-4 bg-card">
          <div className="prose max-w-none text-sm text-foreground dark:text-foreground/90">
            <RemoteAddonReadme readme={changelog} />
          </div>
        </div>
      </>
    )
  }

  return (
    <div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground pt-10">
      <PackageIcon className="w-12 h-12 mb-4 opacity-50" />
      <p className="font-medium">{changelog}</p>
      {changelog === 'No change log was provided' ? (
        <p className="text-xs mt-1">The author hasn't provided release notes.</p>
      ) : (
        <></>
      )}
      {changelog === 'Error loading change log' ? (
        <p className="text-xs mt-0 text-destructive">Could not fetch release details.</p>
      ) : (
        <></>
      )}
    </div>
  )
}
