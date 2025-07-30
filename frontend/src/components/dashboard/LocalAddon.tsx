import { atom, useAtom } from 'jotai'
import { useEffect, useState } from 'react'

import { AddonStatus } from '@/components/dashboard/AddonStatus'
import { UpdateDialogAtomContext } from '@/components/dashboard/contexts'
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
  const isUpdateDialogOpenAtom = atom<boolean>(false)
  const [hasUpdate, setHasUpdate] = useState(false)
  const [latestRelease, setLatestRelease] = useState<Release | null>(null)
  const { latestReleasesMap } = useAddonStore()

  useEffect(() => {
    if (!addon.isManaged) return
    const latestRelease = latestReleasesMap.get(addon.name)
    if (!latestRelease) return
    setHasUpdate(latestRelease.tag_name !== addon.version)
    setLatestRelease(latestRelease)
  }, [latestReleasesMap])

  return (
    <UpdateDialogAtomContext.Provider value={isUpdateDialogOpenAtom}>
      {hasUpdate && latestRelease && (
        <LocalAddonUpdateDialog addon={addon} release={latestRelease} />
      )}
      <LocalAddonContextMenu addon={addon} onOpenChange={setIsContextMenuOpen}>
        <div
          className={cn(
            'cursor-pointer grid grid-cols-4 p-2 hover:bg-muted/50 border-t transition-colors items-center text-sm',
            isContextMenuOpen && 'bg-muted'
          )}
          onClick={() => {
            setSelectedAddon(addon)
            setIsDialogOpen(true)
          }}
        >
          <div className="font-medium">{addon.alias}</div>
          <div className="text-center">{addon.author}</div>
          <div className="text-center">{addon.version}</div>
          <div className="text-center">
            <AddonStatus addon={addon} />
          </div>
        </div>
      </LocalAddonContextMenu>
    </UpdateDialogAtomContext.Provider>
  )
}
