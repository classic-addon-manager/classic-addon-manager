import { useAtom } from 'jotai'
import { BlocksIcon, ChevronRight } from 'lucide-react'
import { useEffect, useState } from 'react'

import { AddonStatus } from '@/components/dashboard/AddonStatus'
import { LocalAddonContextMenu } from '@/components/dashboard/LocalAddonContextMenu'
import { LocalAddonUpdateDialog } from '@/components/dashboard/LocalAddonUpdateDialog'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import type { Addon, Release } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

import { isAddonDialogOpenAtom, selectedAddonAtom } from './atoms'

interface LocalAddonProps {
  addon: Addon
}

export const LocalAddon = ({ addon }: LocalAddonProps) => {
  const [isContextMenuOpen, setIsContextMenuOpen] = useState(false)
  const [, setSelectedAddon] = useAtom(selectedAddonAtom)
  const [, setIsDialogOpen] = useAtom(isAddonDialogOpenAtom)
  const [hasUpdate, setHasUpdate] = useState(false)
  const [latestRelease, setLatestRelease] = useState<Release | null>(null)
  const { latestReleasesMap } = useAddonStore()

  // Try remote icon, fallback to BlocksIcon
  const [hasIcon, setHasIcon] = useState(true)
  const iconUrl =
    'repo' in addon && 'branch' in addon
      ? `https://raw.githubusercontent.com/${addon.repo}/${addon.branch}/icon.png`
      : null

  const Icon = () => {
    if (iconUrl && hasIcon) {
      return (
        <img
          className="h-10 w-10 rounded-lg object-cover border border-border/50 shadow-xs"
          src={iconUrl}
          alt={`${addon.alias} icon`}
          loading="lazy"
          onError={() => setHasIcon(false)}
        />
      )
    }
    return (
      <div className="flex items-center justify-center h-10 w-10 rounded-lg bg-background border border-border/50 shadow-xs">
        <BlocksIcon className="h-5 w-5 opacity-40 stroke-[1.5]" />
      </div>
    )
  }

  useEffect(() => {
    if (!addon.isManaged) return
    const latestRelease = latestReleasesMap.get(addon.name)
    if (!latestRelease) return
    setHasUpdate(latestRelease.tag_name !== addon.version)
    setLatestRelease(latestRelease)
  }, [latestReleasesMap, addon.isManaged, addon.name, addon.version])

  return (
    <>
      {hasUpdate && latestRelease && (
        <LocalAddonUpdateDialog addon={addon} release={latestRelease} />
      )}
      <LocalAddonContextMenu addon={addon} onOpenChange={setIsContextMenuOpen}>
        <div
          className={cn(
            'group grid grid-cols-12 items-center gap-2 bg-muted/30 hover:bg-muted/50 h-16 w-full rounded-xl cursor-pointer transition-all px-4 py-2 border border-border/50 hover:border-primary/30 hover:shadow-sm hover:ring-1 hover:ring-primary/10',
            isContextMenuOpen && 'ring-1 ring-primary/30 bg-muted/50 border-primary/30'
          )}
          onClick={() => {
            setSelectedAddon(addon)
            setIsDialogOpen(true)
          }}
        >
          {/* Left: Icon + Title */}
          <div className="flex items-center gap-3 col-span-4">
            <div className="shrink-0">
              <Icon />
            </div>

            <div className="flex flex-col relative">
              <div className="flex items-center gap-2 text-foreground font-medium group-hover:text-primary transition-colors">
                <span className="truncate">{addon.alias}</span>
                {hasUpdate && (
                  <Badge
                    variant="outline"
                    className="ml-0.5 px-1.5 py-0.5 text-[9px] font-bold rounded-full shadow-sm flex items-center gap-0.5 shrink-0 text-amber-600 border-amber-600/20 bg-amber-500/10"
                  >
                    Outdated
                  </Badge>
                )}
              </div>
              <div className="flex items-center gap-3 text-xs text-muted-foreground mt-0.5">
                {addon.isManaged && (
                  <div className="truncate" title={`Version ${addon.version}`}>
                    {addon.version}
                  </div>
                )}

                {!addon.isManaged && (
                  <div className="px-1.5 py-0.5 rounded-md text-[10px] font-medium tracking-wide bg-secondary/50 border border-secondary text-secondary-foreground shadow-xs">
                    Manual
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Middle: Author */}
          <div
            className="text-sm text-muted-foreground truncate text-center col-span-4 flex items-center justify-center"
            title={`Author: ${addon.author}`}
          >
            {addon.author}
          </div>

          {/* Right: Status */}
          <div className="flex items-center justify-end gap-2 col-span-4">
            <div className="shrink-0">
              <AddonStatus addon={addon} />
            </div>
            <ChevronRight className="h-4 w-4 text-muted-foreground opacity-50 group-hover:opacity-100 transition-opacity" />
          </div>
        </div>
      </LocalAddonContextMenu>
    </>
  )
}
