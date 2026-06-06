import { AnimatePresence, motion } from 'framer-motion'
import { useAtomValue } from 'jotai'
import { PackageSearch } from 'lucide-react'

import { listAnimations } from '@/animations/listAnimations'
import { AddonSkeleton } from '@/components/addons/AddonSkeleton'
import { RemoteAddon } from '@/components/addons/RemoteAddon'
import { usePreferencesStore } from '@/stores/preferencesStore'

import { filteredAddonsAtom, isAddonsReadyAtom, searchQueryAtom } from './atoms'

const listContainerClass = 'flex flex-1 flex-col gap-4 py-4'
const gridContainerClass =
  'grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-3 py-4'

export const AddonList = () => {
  const isReady = useAtomValue(isAddonsReadyAtom)
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
    <AnimatePresence mode="wait">
      {filteredAddons.length === 0 ? (
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
  )
}
