import { useAtom, useSetAtom } from 'jotai'
import { useEffect } from 'react'

import { AddonList } from '@/components/addons/AddonList'
import { AddonsToolbar } from '@/components/addons/AddonsToolbar'
import {
  isManifestDialogOpenAtom,
  loadAddonsAtom,
  selectedManifestAtom,
} from '@/components/addons/atoms'
import { RemoteAddonDialog } from '@/components/addons/RemoteAddonDialog'

export const Addons = () => {
  const loadAddons = useSetAtom(loadAddonsAtom)
  const [selectedManifest, setSelectedManifest] = useAtom(selectedManifestAtom)
  const [isDialogOpen, setDialogOpen] = useAtom(isManifestDialogOpenAtom)

  useEffect(() => {
    loadAddons()
  }, [])

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AddonsToolbar />
      {selectedManifest && (
        <RemoteAddonDialog
          manifest={selectedManifest}
          open={isDialogOpen}
          onOpenChange={open => {
            setDialogOpen(open)
            if (!open) {
              setTimeout(() => setSelectedManifest(null), 200)
            }
          }}
          onViewDependency={manifest => {
            setSelectedManifest(manifest)
            setDialogOpen(true)
          }}
          onAddonInstalled={loadAddons}
          onAddonUninstalled={loadAddons}
        />
      )}

      <div className="min-h-0 flex-1 overflow-auto">
        <div className="container mx-auto px-4">
          <AddonList />
        </div>
      </div>
    </div>
  )
}
