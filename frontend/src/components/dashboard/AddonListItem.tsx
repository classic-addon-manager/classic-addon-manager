import { BlocksIcon, Check, Download, LoaderCircle } from 'lucide-react'
import { useEffect, useState } from 'react'

import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import type { Addon } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

interface AddonListItemProps {
  addon: Addon
  isSelected: boolean
  onClick: () => void
}

export const AddonListItem = ({ addon, isSelected, onClick }: AddonListItemProps) => {
  const [hasUpdate, setHasUpdate] = useState(false)
  const { latestReleasesMap, isCheckingForUpdates } = useAddonStore()

  const [hasIcon, setHasIcon] = useState(true)
  const iconUrl =
    'repo' in addon && 'branch' in addon
      ? `https://raw.githubusercontent.com/${addon.repo}/${addon.branch}/icon.png`
      : null

  useEffect(() => {
    if (!addon.isManaged) return
    const latest = latestReleasesMap.get(addon.name)
    if (!latest) return
    setHasUpdate(latest.tag_name !== addon.version)
  }, [latestReleasesMap, addon.isManaged, addon.name, addon.version])

  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'group flex items-center gap-2.5 w-full rounded-lg px-2.5 py-2 text-left transition-all cursor-pointer border',
        isSelected
          ? 'bg-primary/10 border-primary/30 ring-1 ring-primary/20'
          : 'bg-transparent border-transparent hover:bg-muted/50 hover:border-border/50'
      )}
    >
      <div className="shrink-0">
        {iconUrl && hasIcon ? (
          <img
            className="h-9 w-9 rounded-lg object-cover border border-border/50 shadow-xs"
            src={iconUrl}
            alt={`${addon.alias} icon`}
            loading="lazy"
            onError={() => setHasIcon(false)}
          />
        ) : (
          <div className="flex items-center justify-center h-9 w-9 rounded-lg bg-background border border-border/50 shadow-xs">
            <BlocksIcon className="h-4 w-4 opacity-40 stroke-[1.5]" />
          </div>
        )}
      </div>

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-1.5">
          <span className={cn('truncate text-sm font-medium', isSelected && 'text-primary')}>
            {addon.alias}
          </span>
          {hasUpdate && (
            <Badge
              variant="outline"
              className="ml-0.5 px-1.5 py-0 text-[8px] font-bold rounded-full shrink-0 text-amber-600 border-amber-600/20 bg-amber-500/10"
            >
              Update
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-2 mt-0.5">
          {addon.isManaged ? (
            <span
              className="text-xs text-muted-foreground truncate"
              title={`Version ${addon.version}`}
            >
              {addon.version}
            </span>
          ) : (
            <span className="px-1 py-0.5 rounded text-[9px] font-medium tracking-wide bg-secondary/50 border border-secondary text-secondary-foreground">
              Manual
            </span>
          )}
          <span className="text-xs text-muted-foreground truncate">{addon.author}</span>
        </div>
      </div>

      <div className="shrink-0 flex items-center">
        <AddonListItemIndicator
          addon={addon}
          hasUpdate={hasUpdate}
          isCheckingForUpdates={isCheckingForUpdates}
        />
      </div>
    </button>
  )
}

const AddonListItemIndicator = ({
  addon,
  hasUpdate,
  isCheckingForUpdates,
}: {
  addon: Addon
  hasUpdate: boolean
  isCheckingForUpdates: boolean
}) => {
  if (addon.isManaged && isCheckingForUpdates) {
    return <LoaderCircle className="animate-spin text-primary" size={16} />
  }

  if (addon.isManaged && hasUpdate) {
    return <Download size={16} className="text-amber-600" />
  }

  if (addon.isManaged) {
    return <Check size={16} className="text-green-600" />
  }

  return <span className="w-2 h-2 rounded-full bg-orange-500/60" />
}
