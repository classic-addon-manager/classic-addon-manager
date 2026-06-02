import { AnimatePresence, motion } from 'framer-motion'
import { useSetAtom } from 'jotai'
import { Blocks, LoaderCircle, Package, PackageSearch, Upload } from 'lucide-react'
import { useEffect } from 'react'

import { listAnimations } from '@/animations/listAnimations'
import { activePageAtom } from '@/atoms/sidebarAtoms'
import { AddonDetailsPane } from '@/components/dashboard/AddonDetailsPane'
import { AddonListItem } from '@/components/dashboard/AddonListItem'
import { versionSelectAtom } from '@/components/dashboard/atoms'
import { DashboardToolbar } from '@/components/dashboard/DashboardToolbar'
import { LocalAddonVersionSelectDialog } from '@/components/dashboard/index'
import { LocalAddonUpdateDialog } from '@/components/dashboard/LocalAddonUpdateDialog'
import { useDashboardController } from '@/components/dashboard/useDashboardController'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import type { Addon } from '@/lib/wails'
import { useAddonStore } from '@/stores/addonStore'

const NoAddonsInstalled = ({ onInstallZip }: { onInstallZip: () => void }) => {
  const setActiveItem = useSetAtom(activePageAtom)

  return (
    <div className="flex flex-col items-center justify-center h-full p-8 text-center">
      <div className="rounded-full bg-primary/10 p-4 mb-4">
        <Package className="size-8 text-primary" />
      </div>
      <h3 className="text-xl font-semibold tracking-tight mb-2">No Addons Installed</h3>
      <p className="text-muted-foreground text-sm max-w-sm mb-6">
        Get started by browsing our collection of addons or install from a ZIP file.
      </p>
      <div className="flex gap-3">
        <Button size="default" onClick={() => setActiveItem('addons')} className="cursor-pointer">
          <Blocks className="mr-2 size-4" />
          Browse Addons
        </Button>
        <Button size="default" variant="outline" onClick={onInstallZip} className="cursor-pointer">
          <Upload className="mr-2 size-4" />
          Install ZIP
        </Button>
      </div>
    </div>
  )
}

const NoSelectedAddon = () => (
  <div className="flex flex-col items-center justify-center h-full p-8 text-center">
    <div className="rounded-full bg-muted p-4 mb-4">
      <Blocks className="size-8 text-muted-foreground" />
    </div>
    <h3 className="text-lg font-semibold tracking-tight mb-1">Select an addon</h3>
    <p className="text-muted-foreground text-sm max-w-xs">
      Choose an addon from the list to view its details, actions, and description.
    </p>
  </div>
)

export const Dashboard = () => {
  const {
    isLoading,
    filteredAddons,
    selectedAddon,
    setSelectedAddon,
    versionSelectAddon,
    handleInstallZip,
  } = useDashboardController()

  const { installedAddons, latestReleasesMap } = useAddonStore()
  const openVersionSelect = useSetAtom(versionSelectAtom)

  useEffect(() => {
    if (!selectedAddon && filteredAddons.length > 0) {
      setSelectedAddon(filteredAddons[0])
    }
  }, [filteredAddons, selectedAddon, setSelectedAddon])

  useEffect(() => {
    if (selectedAddon) {
      const stillExists = filteredAddons.find(a => a.name === selectedAddon.name)
      if (!stillExists && filteredAddons.length > 0) {
        setSelectedAddon(filteredAddons[0])
      } else if (!stillExists) {
        setSelectedAddon(null)
      }
    }
  }, [filteredAddons, selectedAddon, setSelectedAddon])

  const handleSelectAddon = (addon: Addon) => {
    setSelectedAddon(addon)
  }

  const latestReleaseForSelected = selectedAddon && latestReleasesMap.get(selectedAddon.name)

  if (isLoading) {
    return (
      <div className="flex h-full min-h-0 items-center justify-center bg-background">
        <div className="relative">
          <div className="absolute -inset-8 rounded-full bg-primary/10 blur-xl animate-pulse" />
          <div className="absolute -inset-4 rounded-full bg-primary/20 animate-ping opacity-75" />
          <LoaderCircle className="size-12 animate-spin text-primary relative" strokeWidth={1.5} />
        </div>
        <p className="text-muted-foreground ml-6 animate-pulse">Loading your addons...</p>
      </div>
    )
  }

  if (installedAddons.length === 0) {
    return (
      <div className="flex h-full min-h-0 flex-col bg-background">
        <DashboardToolbar />
        <main className="min-h-0 flex-1 overflow-auto">
          <NoAddonsInstalled onInstallZip={handleInstallZip} />
        </main>
      </div>
    )
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-background">
      <DashboardToolbar />
      {selectedAddon &&
        latestReleaseForSelected &&
        latestReleaseForSelected.published_at > selectedAddon.updatedAt && (
          <LocalAddonUpdateDialog addon={selectedAddon} release={latestReleaseForSelected} />
        )}

      {versionSelectAddon && <LocalAddonVersionSelectDialog addon={versionSelectAddon} />}

      <main className="flex min-h-0 flex-1 overflow-hidden">
        <div className="w-70 shrink-0 border-r flex flex-col h-full">
          <ScrollArea className="flex-1 min-h-0">
            <div className="p-2">
              <AnimatePresence initial={false}>
                {filteredAddons.length === 0 ? (
                  <motion.div
                    key="no-results"
                    className="flex flex-col items-center justify-center p-8 text-center min-h-50"
                    {...listAnimations.emptyState}
                  >
                    <PackageSearch className="h-10 w-10 mb-3 text-muted-foreground" />
                    <h3 className="text-sm font-semibold mb-1">No addons match your search</h3>
                    <p className="text-xs text-muted-foreground">
                      Try a different query or clear the search.
                    </p>
                  </motion.div>
                ) : (
                  <AnimatePresence initial={false}>
                    {filteredAddons.map(addon => (
                      <motion.div
                        key={addon.name}
                        layout
                        {...listAnimations.item}
                        className="mb-1.5 last:mb-0"
                      >
                        <AddonListItem
                          addon={addon}
                          isSelected={selectedAddon?.name === addon.name}
                          onClick={() => handleSelectAddon(addon)}
                        />
                      </motion.div>
                    ))}
                  </AnimatePresence>
                )}
              </AnimatePresence>
            </div>
          </ScrollArea>
        </div>

        <div className="w-0 grow overflow-hidden">
          {selectedAddon ? (
            <AddonDetailsPane addon={selectedAddon} onOpenVersionSelect={openVersionSelect} />
          ) : (
            <NoSelectedAddon />
          )}
        </div>
      </main>
    </div>
  )
}
