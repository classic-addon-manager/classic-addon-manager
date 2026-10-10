import { CheckIcon } from 'lucide-react'

import { AddonIcon } from '@/components/shared/AddonIcon'

import { ActionButton } from './ActionButton'
import { listRowGridClass } from './listLayout'
import { DownloadMeter, LikeRing } from './ListStats'
import { TagDots } from './TagDots'
import type { RemoteAddonItemProps } from './types'
import { useRemoteAddonItem } from './useRemoteAddonItem'
import { WarningIcon } from './WarningIcon'

export const ListHeader = () => (
  <div
    className={`${listRowGridClass} px-3 pb-1 text-[10px] uppercase tracking-widest text-muted-foreground/70`}
  >
    <span />
    <span>Addon</span>
    <span className="hidden md:block">Tags</span>
    <span className="text-right">Downloads</span>
    <span className="text-right">Liked</span>
    <span />
  </div>
)

export const List = ({ manifest, installed }: RemoteAddonItemProps) => {
  const { isNew, iconUrl, openDialog } = useRemoteAddonItem({
    manifest,
    installed,
  })

  return (
    <div
      className={`${listRowGridClass} group relative w-full cursor-pointer px-3 py-2.5 transition-colors hover:bg-muted/50 before:absolute before:inset-y-2.5 before:left-0 before:w-0.5 before:rounded-full before:bg-primary before:scale-y-0 before:transition-transform hover:before:scale-y-100`}
      onClick={openDialog}
    >
      <div className="relative">
        <AddonIcon name={manifest.alias} src={iconUrl} />
        {installed && (
          <span className="absolute -bottom-1 -right-1 flex h-4 w-4 items-center justify-center rounded-full border-2 border-background bg-green-500 text-background">
            <CheckIcon className="h-2 w-2 stroke-[4]" />
          </span>
        )}
      </div>

      <div className="min-w-0">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate font-medium text-foreground transition-colors group-hover:text-primary">
            {manifest.alias}
          </span>
          {isNew && (
            <span className="shrink-0 rounded-sm bg-primary px-1.5 py-0.5 text-[10px] font-semibold leading-none tracking-wider text-primary-foreground">
              NEW
            </span>
          )}
          <WarningIcon warning={manifest.warning} chip />
        </div>
        <div className="mt-0.5 truncate text-xs text-muted-foreground/70">
          <span className="text-muted-foreground" title={`Author: ${manifest.author}`}>
            {manifest.author}
          </span>
          {manifest.description && ` · ${manifest.description}`}
        </div>
      </div>

      <div className="hidden min-w-0 md:block">
        <TagDots tags={manifest.tags} maxTags={3} />
      </div>

      <DownloadMeter downloads={manifest.downloads} />
      <LikeRing percentage={manifest.like_percentage} />

      <div className="flex justify-end">
        <ActionButton installed={installed} />
      </div>
    </div>
  )
}
