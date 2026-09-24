import { AnimatePresence, motion } from 'framer-motion'
import { useAtomValue, useSetAtom } from 'jotai'
import { AlertTriangle, PackageSearch } from 'lucide-react'

import { listAnimations } from '@/animations/listAnimations'
import { AddonSkeleton } from '@/components/addons/AddonSkeleton'
import { RemoteAddon } from '@/components/addons/RemoteAddon'
import { Button } from '@/components/ui/button'
import { usePreferencesStore } from '@/stores/preferencesStore'

import {
  addonsAtom,
  addonsErrorAtom,
  filteredAddonsAtom,
  isAddonsReadyAtom,
  loadAddonsAtom,
  searchQueryAtom,
} from './atoms'

const listContainerClass = 'flex flex-1 flex-col gap-4 py-4'
const gridContainerClass =
  'grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-3 py-4'

export const AddonList = () => {
  const isReady = useAtomValue(isAddonsReadyAtom)
  const addonsError = useAtomValue(addonsErrorAtom)
  const loadAddons = useSetAtom(loadAddonsAtom)
  const addons = useAtomValue(addonsAtom)
  const filteredAddons = useAtomValue(filteredAddonsAtom)
  const searchQuery = useAtomValue(searchQueryAtom)
  const viewMode = usePreferencesStore(s => s.addonViewMode)

  const containerClass = viewMode === 'grid' ? gridContainerClass : listContainerClass

  if (!isReady) {
    return (
      <div
        className={
          viewMode === 'grid'
            ? `${gridContainerClass} relative flex-1`
            : `${listContainerClass} relative flex-1`
        }
      >
        {Array(7)
          .fill(null)
          .map((_, i) => (
            <AddonSkeleton key={i} variant={viewMode} />
          ))}
      </div>
    )
  }

  return (
    <>
      {addonsError && addons.length > 0 && (
        <div className="mt-4 flex items-center gap-2 rounded-lg border bg-card px-3 py-2 text-sm text-muted-foreground">
          <AlertTriangle className="h-4 w-4 shrink-0 text-orange-500" />
          <span className="min-w-0 flex-1">
            Couldn't refresh addons: {addonsError.substring(0, 120)}. Showing the last loaded
            list.
          </span>
          <Button variant="ghost" size="sm" onClick={() => loadAddons(true)}>
            Retry
          </Button>
        </div>
      )}
      <AnimatePresence mode="wait">
        {addonsError && addons.length === 0 ? (
          <motion.div
            key="addons-error"
            className="flex flex-col items-center justify-center py-16 text-center text-muted-foreground"
            {...listAnimations.emptyState}
          >
            <AlertTriangle className="h-12 w-12 mb-4" />
            <h3 className="text-lg font-semibold mb-2">Couldn't load addons</h3>
            <p className="max-w-sm">{addonsError.substring(0, 200)}</p>
            <Button
              variant="outline"
              size="sm"
              className="mt-4"
              onClick={() => loadAddons(true)}
            >
              Retry
            </Button>
          </motion.div>
        ) : filteredAddons.length === 0 ? (
          <motion.div
            key="no-addons"
            className="flex flex-col items-center justify-center py-16 text-center text-muted-foreground"
            {...listAnimations.emptyState}
          >
            <PackageSearch className="h-12 w-12 mb-4" />
            <h3 className="text-lg font-semibold mb-2">No addons found</h3>
            <p className="max-w-sm">
              {searchQuery
                ? 'No addons match your search criteria. Try adjusting your search or filters.'
                : 'No addons are currently available. Check back later or try refreshing.'}
            </p>
          </motion.div>
        ) : (
          <motion.main
            key={`addon-list-${viewMode}`}
            className="flex-1"
            {...listAnimations.container}
          >
            <motion.div className={containerClass} layout>
              <AnimatePresence initial={false}>
                {filteredAddons.map(addon => (
                  <motion.div key={addon.manifest.name} layout {...listAnimations.item}>
                    <RemoteAddon
                      manifest={addon.manifest}
                      installed={addon.isInstalled}
                      variant={viewMode}
                    />
                  </motion.div>
                ))}
              </AnimatePresence>
            </motion.div>
          </motion.main>
        )}
      </AnimatePresence>
    </>
  )
}
