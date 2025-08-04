import { AnimatePresence, motion } from 'framer-motion'
import { useAtomValue } from 'jotai'
import { PackageSearch } from 'lucide-react'

import { AddonSkeleton } from '@/components/addons/AddonSkeleton'
import { RemoteAddon } from '@/components/addons/RemoteAddon'

import { filteredAddonsAtom, isAddonsReadyAtom, searchQueryAtom } from './atoms'

export const AddonList = () => {
  const isReady = useAtomValue(isAddonsReadyAtom)
  const filteredAddons = useAtomValue(filteredAddonsAtom)
  const searchQuery = useAtomValue(searchQueryAtom)

  if (!isReady) {
    return (
      <div className="flex flex-1 flex-col gap-4 py-4 relative">
        {Array(7)
          .fill(null)
          .map(() => (
            <AddonSkeleton />
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
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.3, ease: 'easeOut' }}
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
          key="addon-list"
          className="flex-1 overflow-auto"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.3, ease: 'easeOut' }}
        >
          <div className="flex flex-1 flex-col gap-4 py-4">
            {filteredAddons.map(addon => (
              <RemoteAddon
                key={addon.manifest.name}
                manifest={addon.manifest}
                installed={addon.isInstalled}
              />
            ))}
          </div>
        </motion.main>
      )}
    </AnimatePresence>
  )
}
