import { useAtom } from 'jotai'
import { BlocksIcon, ChevronRight } from 'lucide-react'
import { useEffect, useState } from 'react'

import { AddonStatus } from '@/components/dashboard/AddonStatus'
import { LocalAddonContextMenu } from '@/components/dashboard/LocalAddonContextMenu'
import { LocalAddonUpdateDialog } from '@/components/dashboard/LocalAddonUpdateDialog'
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
          className="h-10 w-10 rounded-md object-cover"
          src={iconUrl}
          alt={`${addon.alias} icon`}
          loading="lazy"
          onError={() => setHasIcon(false)}
        />
      )
    }
    return (
      <div className="flex items-center justify-center h-10 w-10 rounded-md bg-background border">
        <BlocksIcon className="h-5 w-5 opacity-50 stroke-2" />
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
            'group grid grid-cols-12 items-center gap-2 bg-muted/50 hover:bg-muted/70 h-16 w-full rounded-lg cursor-pointer transition-all px-3 py-2',
            isContextMenuOpen && 'ring-1 ring-primary/30 bg-muted/70'
          )}
          onClick={() => {
            setSelectedAddon(addon)
            setIsDialogOpen(true)
          }}
        >
          {/* Left: Icon + Title */}
          <div className="flex items-center gap-3 col-span-4">
            <div className="relative shrink-0">
              <Icon />
              {hasUpdate && (
                <div className="absolute -top-1.5 -right-1.5 bg-red-500 text-white text-[8px] font-bold px-1 py-0.5 rounded-full shadow-md">
                  UPD
                </div>
              )}
            </div>

            <div className="flex flex-col overflow-hidden">
              <div className="text-foreground font-medium truncate group-hover:text-primary transition-colors">
                {addon.alias}
              </div>
              <div className="flex items-center gap-3 text-xs text-muted-foreground mt-0.5">
                {addon.isManaged && (
                  <div className="truncate" title={`Version ${addon.version}`}>
                    {addon.version}
                  </div>
                )}

                {!addon.isManaged && (
                  <div className="px-1.5 py-0.5 rounded text-[10px] font-medium tracking-wide bg-background border text-muted-foreground">
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
