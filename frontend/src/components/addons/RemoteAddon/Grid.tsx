import { CheckIcon } from 'lucide-react'

import { AddonIcon } from '@/components/shared/AddonIcon'
import { DeprecatedIcon } from '@/components/shared/DeprecatedIcon'
import { deprecationOf } from '@/lib/deprecation'

import { ActionButton } from './ActionButton'
import { DownloadMeter, LikeRing } from './ListStats'
import { TagDots } from './TagDots'
import type { RemoteAddonItemProps } from './types'
import { useRemoteAddonItem } from './useRemoteAddonItem'
import { WarningIcon } from './WarningIcon'

export const Grid = ({ manifest, installed }: RemoteAddonItemProps) => {
  const { isNew, iconUrl, openDialog, prefetchHandlers } = useRemoteAddonItem({
    manifest,
    installed,
  })
  const deprecation = deprecationOf(manifest)

  return (
    <div
      className="group flex h-full cursor-pointer flex-col overflow-hidden rounded-xl border border-border/60 bg-muted/20 transition-colors hover:border-primary/35 hover:bg-muted/50"
      onClick={openDialog}
      {...prefetchHandlers}
    >
      <div className="flex min-w-0 items-center gap-2.5 px-3 pt-3">
        <div className="relative shrink-0">
          <AddonIcon name={manifest.alias} src={iconUrl} />
          {installed && (
            <span className="absolute -bottom-1 -right-1 flex h-4 w-4 items-center justify-center rounded-full border-2 border-background bg-green-500 text-background">
              <CheckIcon className="h-2 w-2 stroke-[4]" />
            </span>
          )}
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex min-w-0 items-center gap-1.5">
            <span className="truncate text-sm font-medium text-foreground transition-colors group-hover:text-primary">
              {manifest.alias}
            </span>
            {isNew && (
              <span className="shrink-0 rounded-sm bg-primary px-1.5 py-0.5 text-[10px] font-semibold leading-none tracking-wider text-primary-foreground">
                NEW
              </span>
            )}
            {deprecation ? (
              <DeprecatedIcon deprecation={deprecation} compact />
            ) : (
              <WarningIcon warning={manifest.warning} compact />
            )}
          </div>
          <div
            className="truncate text-xs text-muted-foreground"
            title={`Author: ${manifest.author}`}
          >
            {manifest.author}
          </div>
        </div>
      </div>

      <p className="mt-2.5 line-clamp-2 min-h-8 px-3 text-xs leading-4 text-muted-foreground/70">
        {manifest.description}
      </p>

      <div className="mb-2.5 mt-2 flex min-h-5 items-center px-3">
        <TagDots tags={manifest.tags} maxTags={3} />
      </div>

      <div className="mt-auto flex h-10 items-center gap-3 border-t border-border/60 px-3">
        <DownloadMeter downloads={manifest.downloads} />
        <LikeRing percentage={manifest.like_percentage} fixedWidth={false} />
        <div className="ml-auto flex shrink-0 whitespace-nowrap">
          <ActionButton installed={installed} deprecated={deprecation !== null} compact />
        </div>
      </div>
    </div>
  )
}
